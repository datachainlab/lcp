use crate::dcap::INTEL_ROOT_CA;
use crate::errors::Error;
use attestation_report::DCAPQuote;
use dcap_quote_verifier::cert::{get_x509_subject_cn, parse_certchain};
use dcap_quote_verifier::collaterals::IntelCollateral;
use dcap_quote_verifier::sgx_extensions::extract_sgx_extensions;
use dcap_quote_verifier::types::quotes::CertData;
use dcap_quote_verifier::types::utils::parse_pem;
use dcap_quote_verifier::verifier::QuoteVerificationOutput;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;
use log::info;

#[derive(Debug)]
pub struct CollateralService {
    pub pccs_url: String,
    pub certs_service_url: String,
    pub is_early_update: bool,
}

impl CollateralService {
    pub fn new(pccs_ur: &str, certs_service_url: &str, is_early_update: bool) -> Self {
        CollateralService {
            pccs_url: pccs_ur.to_string(),
            certs_service_url: certs_service_url.to_string(),
            is_early_update,
        }
    }

    pub fn get_collateral(&self, qe_cert_data: &CertData) -> Result<IntelCollateral, Error> {
        let pccs_url = self.pccs_url.trim_end_matches('/');
        let certs_service_url = self.certs_service_url.trim_end_matches('/');
        let base_url = format!("{pccs_url}/sgx/certification/v4");
        if qe_cert_data.cert_data_type != 5 {
            return Err(Error::collateral("QE Cert Type must be 5".to_string()));
        }
        let certchain_pems = parse_pem(&qe_cert_data.cert_data)
            .map_err(|e| Error::collateral(format!("cannot parse QE cert chain: {}", e)))?;

        let certchain = parse_certchain(&certchain_pems).map_err(Error::dcap_quote_verifier)?;
        if certchain.len() != 3 {
            return Err(Error::collateral(
                "QE Cert chain must have 3 certs".to_string(),
            ));
        }

        let update_policy = if self.is_early_update {
            "early"
        } else {
            "standard"
        };

        // get the pck certificate
        let pck_cert = &certchain[0];
        let pck_cert_issuer = &certchain[1];

        // get the SGX extension
        let sgx_extensions =
            extract_sgx_extensions(pck_cert).map_err(Error::dcap_quote_verifier)?;
        let (tcbinfo_bytes, sgx_tcb_signing_der) = {
            let fmspc = hex::encode_upper(sgx_extensions.fmspc);
            let res = http_get(format!(
                "{base_url}/tcb?fmspc={fmspc}&update={update_policy}"
            ))?;
            let issuer_chain =
                extract_raw_certs(get_header(&res, "TCB-Info-Issuer-Chain")?.as_bytes())?;
            (res.bytes()?.to_vec(), issuer_chain[0].clone())
        };

        let qeidentity_bytes = http_get(format!("{base_url}/qe/identity?update={update_policy}"))?
            .bytes()?
            .to_vec();
        let sgx_intel_root_ca_crl_der =
            http_get(format!("{certs_service_url}/IntelSGXRootCA.der"))?
                .bytes()?
                .to_vec();

        let pck_crl_url = match get_x509_subject_cn(pck_cert_issuer).as_str() {
            "Intel SGX PCK Platform CA" => format!("{base_url}/pckcrl?ca=platform&encoding=der"),
            "Intel SGX PCK Processor CA" => format!("{base_url}/pckcrl?ca=processor&encoding=der"),
            cn => {
                return Err(Error::collateral(format!(
                    "Unknown PCK Cert Subject CN: {}",
                    cn
                )));
            }
        };
        let sgx_pck_crl_der = http_get(pck_crl_url)?.bytes()?.to_vec();

        Ok(IntelCollateral {
            tcbinfo_bytes,
            qeidentity_bytes,
            sgx_intel_root_ca_der: INTEL_ROOT_CA.to_vec(),
            sgx_tcb_signing_der,
            sgx_intel_root_ca_crl_der,
            sgx_pck_crl_der,
        })
    }
}

fn get_header(res: &reqwest::blocking::Response, name: &str) -> Result<String, Error> {
    let value = res
        .headers()
        .get(name)
        .ok_or_else(|| Error::collateral(format!("missing header {}", name)))?
        .to_str()
        .map_err(|e| Error::collateral(format!("invalid header value: {}", e)))?;
    let value = urlencoding::decode(value)
        .map_err(|e| Error::collateral(format!("invalid header value: {}", e)))?;
    Ok(value.into_owned())
}

fn extract_raw_certs(cert_chain: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    Ok(pem::parse_many(cert_chain)
        .map_err(Error::pem)?
        .iter()
        .map(|i| i.contents().to_vec())
        .collect())
}

fn http_get(url: String) -> Result<reqwest::blocking::Response, Error> {
    info!("get collateral from {}", url);
    let res = reqwest::blocking::get(&url).map_err(Error::reqwest_get)?;
    if !res.status().is_success() {
        return Err(Error::invalid_http_status(url, res.status()));
    }
    Ok(res)
}

#[derive(Debug)]
pub struct DCAPRemoteAttestationResult {
    pub raw_quote: Vec<u8>,
    pub output: QuoteVerificationOutput,
    pub collateral: IntelCollateral,
}

impl DCAPRemoteAttestationResult {
    pub fn get_ra_quote(&self, attested_at: Time) -> DCAPQuote {
        DCAPQuote {
            raw: self.raw_quote.clone(),
            fmspc: self.output.fmspc,
            tcb_status: self.output.tcb_status.to_string(),
            advisory_ids: self.output.advisory_ids.clone(),
            attested_at,
            collateral: DcapCollateral {
                tcbinfo_bytes: self.collateral.tcbinfo_bytes.clone(),
                qeidentity_bytes: self.collateral.qeidentity_bytes.clone(),
                sgx_intel_root_ca_der: self.collateral.sgx_intel_root_ca_der.clone(),
                sgx_tcb_signing_der: self.collateral.sgx_tcb_signing_der.clone(),
                sgx_intel_root_ca_crl_der: self.collateral.sgx_intel_root_ca_crl_der.clone(),
                sgx_pck_crl_der: self.collateral.sgx_pck_crl_der.clone(),
            },
        }
    }
}

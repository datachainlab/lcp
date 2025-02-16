use attestation_report::DCAPQuote;
use dcap_quote_verifier::collaterals::IntelCollateral;
use dcap_quote_verifier::verifier::VerifiedOutput;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;

#[derive(Debug)]
pub struct DCAPRemoteAttestationResult {
    pub raw_quote: Vec<u8>,
    pub output: VerifiedOutput,
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

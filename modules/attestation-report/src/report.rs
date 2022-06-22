#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use chrono::prelude::DateTime;
use log::*;
use pem;
use serde_json::Value;
use sgx_types::{sgx_quote_t, sgx_status_t};
use std::ptr;
use std::string::String;
use std::time::SystemTime;
#[cfg(feature = "sgx")]
use std::untrusted::time::SystemTimeEx;
use std::vec::Vec;

pub const IAS_REPORT_CA: &[u8] =
    include_bytes!("../../../enclave/Intel_SGX_Attestation_RootCA.pem");

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];
static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

/// AttestationReport can be endorsed by either the Intel Attestation Service
/// using EPID or Data Center Attestation
/// Service (platform dependent) using ECDSA.
#[derive(Default)]
pub struct EndorsedAttestationReport {
    /// Attestation report generated by the hardware
    pub report: Vec<u8>,
    /// Singature of the report
    pub signature: Vec<u8>,
    /// Certificate matching the signing key of the signature
    pub signing_cert: Vec<u8>,
}

impl EndorsedAttestationReport {
    #[cfg(not(feature = "sgx"))]
    pub fn read_from_file(filepath: &str) -> Result<Self, sgx_status_t> {
        use std::fs::File;
        use std::io::Read;
        let mut file = File::open(filepath).unwrap();
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf).unwrap();
        Ok(Self {
            report: buf,
            signature: vec![],
            signing_cert: vec![],
        })
    }

    #[cfg(feature = "sgx")]
    pub fn write_to_file(&self, filepath: &str) -> Result<(), sgx_status_t> {
        use enclave_utils::storage::write_to_untrusted;
        if let Err(e) = write_to_untrusted(&self.report, filepath) {
            Err(sgx_status_t::SGX_ERROR_FILE_CANT_WRITE_RECOVERY_FILE)
        } else {
            Ok(())
        }
    }
}

pub fn verify_report(report: &EndorsedAttestationReport) -> Result<(), sgx_status_t> {
    let now = match webpki::Time::try_from(SystemTime::now()) {
        Ok(r) => r,
        Err(e) => {
            error!("webpki::Time::try_from failed with {:?}", e);
            return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
        }
    };

    let root_ca_pem = pem::parse(IAS_REPORT_CA).expect("failed to parse pem bytes");
    let root_ca = root_ca_pem.contents;

    let mut root_store = rustls::RootCertStore::empty();
    root_store
        .add(&rustls::Certificate(root_ca.clone()))
        .unwrap();

    let trust_anchors: Vec<webpki::TrustAnchor> = root_store
        .roots
        .iter()
        .map(|cert| cert.to_trust_anchor())
        .collect();

    let mut chain: Vec<&[u8]> = Vec::new();
    chain.push(&root_ca);

    let report_cert = webpki::EndEntityCert::from(&report.signing_cert).unwrap();

    match report_cert.verify_is_valid_tls_server_cert(
        SUPPORTED_SIG_ALGS,
        &webpki::TLSServerTrustAnchors(&trust_anchors),
        &chain,
        now,
    ) {
        Ok(r) => r,
        Err(e) => {
            error!("verify_is_valid_tls_server_cert failed with {:?}", e);
            return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
        }
    };

    match report_cert.verify_signature(
        &webpki::RSA_PKCS1_2048_8192_SHA256,
        &report.report,
        &report.signature,
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("verify_signature failed with {:?}", e);
            Err(sgx_status_t::SGX_ERROR_UNEXPECTED)
        }
    }
}

pub struct Quote {
    pub raw: sgx_quote_t,
    pub status: String,
    pub timestamp: i64,
}

pub fn parse_quote_from_report(attn_report: &[u8]) -> Result<Quote, sgx_status_t> {
    let attn_report: Value = serde_json::from_slice(attn_report).unwrap();

    let timestamp = if let Value::String(time) = &attn_report["timestamp"] {
        let time_fixed = time.clone() + "+0000";
        DateTime::parse_from_str(&time_fixed, "%Y-%m-%dT%H:%M:%S%.f%z")
            .unwrap()
            .timestamp()
    } else {
        error!("Failed to fetch timestamp from attestation report");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    };

    if let Value::String(version) = &attn_report["version"] {
        if version != "4" {
            return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
        }
    }

    let quote_status = if let Value::String(quote_status) = &attn_report["isvEnclaveQuoteStatus"] {
        match quote_status.as_ref() {
            "OK" => (),
            "GROUP_OUT_OF_DATE"
            | "GROUP_REVOKED"
            | "SW_HARDENING_NEEDED"
            | "CONFIGURATION_NEEDED"
            | "CONFIGURATION_AND_SW_HARDENING_NEEDED" => (),
            _ => {
                return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
            }
        }
        quote_status
    } else {
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    };

    match &attn_report["isvEnclaveQuoteBody"] {
        Value::String(quote_raw) => {
            let quote = base64::decode(&quote_raw).unwrap();

            let sgx_quote: sgx_quote_t = unsafe { ptr::read(quote.as_ptr() as *const _) };
            Ok(Quote {
                raw: sgx_quote,
                status: quote_status.into(),
                timestamp,
            })
        }
        _ => {
            error!("Failed to fetch isvEnclaveQuoteBody from attestation report");
            Err(sgx_status_t::SGX_ERROR_UNEXPECTED)
        }
    }
}

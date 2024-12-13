use crate::prelude::*;
use crate::serde_base64;
use crate::{errors::Error, Quote};
use base64::{engine::general_purpose::STANDARD as Base64Std, Engine};
use chrono::prelude::DateTime;
use core::fmt::Debug;
use lcp_types::{nanos_to_duration, Time};
use serde::{Deserialize, Serialize};
use sgx_types::sgx_quote_t;

pub const IAS_REPORT_CA: &[u8] =
    include_bytes!("../../../enclave/Intel_SGX_Attestation_RootCA.pem");

static SUPPORTED_SIG_ALGS: &[&webpki::SignatureAlgorithm] = &[
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

/// IASSignedReport represents the signed attestation verification report from Intel
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IASSignedReport {
    /// A report generated by the Intel Attestation Service
    pub avr: String,
    /// Signature of the report
    #[serde(with = "serde_base64")]
    pub signature: Vec<u8>,
    /// Certificate matching the signing key of the signature
    #[serde(with = "serde_base64")]
    pub signing_cert: Vec<u8>,
}

impl IASSignedReport {
    pub fn get_avr(&self) -> Result<IASAttestationVerificationReport, Error> {
        serde_json::from_slice(self.avr.as_ref()).map_err(Error::serde_json)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }
}

// IASAttestationVerificationReport represents Intel's Attestation Verification Report
// https://api.trustedservices.intel.com/documents/sgx-attestation-api-spec.pdf
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IASAttestationVerificationReport {
    pub id: String,
    pub timestamp: String,
    pub version: i64,
    #[serde(alias = "isvEnclaveQuoteStatus")]
    pub isv_enclave_quote_status: String,
    #[serde(alias = "isvEnclaveQuoteBody")]
    pub isv_enclave_quote_body: String,
    #[serde(alias = "revocationReason")]
    pub revocation_reason: Option<i64>,
    #[serde(alias = "pseManifestStatus")]
    pub pse_manifest_status: Option<i64>,
    #[serde(alias = "pseManifestHash")]
    pub pse_manifest_hash: Option<String>,
    #[serde(alias = "platformInfoBlob")]
    pub platform_info_blob: Option<String>,
    pub nonce: Option<String>,
    #[serde(alias = "epidPseudonym")]
    pub epid_pseudonym: Option<Vec<u8>>,
    #[serde(alias = "advisoryURL")]
    pub advisory_url: String,
    #[serde(alias = "advisoryIDs")]
    pub advisory_ids: Vec<String>,
}

impl IASAttestationVerificationReport {
    pub fn attestation_time(&self) -> Result<Time, Error> {
        let time_fixed = self.timestamp.clone() + "+0000";
        let dt = DateTime::parse_from_str(&time_fixed, "%Y-%m-%dT%H:%M:%S%.f%z")?;
        Ok(Time::from_unix_timestamp(
            dt.timestamp(),
            dt.timestamp_subsec_nanos(),
        )?)
    }

    pub fn parse_quote(&self) -> Result<Quote, Error> {
        if self.version != 4 {
            return Err(Error::unexpected_attestation_report_version(
                4,
                self.version,
            ));
        }

        let quote = Base64Std
            .decode(&self.isv_enclave_quote_body)
            .map_err(Error::base64)?;
        let sgx_quote: sgx_quote_t = unsafe { core::ptr::read(quote.as_ptr() as *const _) };
        Ok(Quote {
            raw: sgx_quote,
            status: self.isv_enclave_quote_status.clone(),
            attestation_time: self.attestation_time()?,
        })
    }

    #[cfg(feature = "std")]
    pub fn to_canonical_json(&self) -> Result<String, Error> {
        if self.version != 4 {
            return Err(Error::unexpected_attestation_report_version(
                4,
                self.version,
            ));
        }
        Ok(format!(
            "{}",
            serde_json::json!({
                "id": self.id,
                "timestamp": self.timestamp,
                "version": self.version,
                "advisoryURL": self.advisory_url,
                "advisoryIDs": self.advisory_ids,
                "isvEnclaveQuoteStatus": self.isv_enclave_quote_status,
                "platformInfoBlob": self.platform_info_blob,
                "isvEnclaveQuoteBody": self.isv_enclave_quote_body
            })
        ))
    }
}

pub fn verify_ias_report(current_timestamp: Time, report: &IASSignedReport) -> Result<(), Error> {
    // NOTE: Currently, webpki::Time's constructor only accepts seconds as unix timestamp.
    // Therefore, the current time are rounded up conservatively.
    let duration = nanos_to_duration(current_timestamp.as_unix_timestamp_nanos())?;
    let secs = if duration.subsec_nanos() > 0 {
        duration.as_secs() + 1
    } else {
        duration.as_secs()
    };
    let now = webpki::Time::from_seconds_since_unix_epoch(secs);
    let root_ca_pem = pem::parse(IAS_REPORT_CA).expect("failed to parse pem bytes");
    let root_ca = root_ca_pem.contents();

    let trust_anchors = vec![webpki::TrustAnchor::try_from_cert_der(root_ca)
        .map_err(|e| Error::web_pki(e.to_string()))?];

    let intermediate_certs = vec![root_ca];
    let report_cert = webpki::EndEntityCert::try_from(report.signing_cert.as_slice())
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_is_valid_tls_server_cert(
            SUPPORTED_SIG_ALGS,
            &webpki::TlsServerTrustAnchors(&trust_anchors),
            &intermediate_certs,
            now,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_signature(
            &webpki::RSA_PKCS1_2048_8192_SHA256,
            report.avr.as_ref(),
            &report.signature,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    Ok(())
}

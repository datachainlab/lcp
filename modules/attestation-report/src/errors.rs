#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use std::string::String;

#[derive(Debug, thiserror::Error)]
pub enum AttestationReportError {
    #[error("InvalidAttestationReport: {0}")]
    InvalidAttestationReportError(String),
    #[error("UnexpectedAttestationReportVersionError: expected={0} actual={1}")]
    UnexpectedAttestationReportVersionError(i64, i64),
    #[error("InvalidReportDataError: {0}")]
    InvalidReportDataError(String),
    #[error("WebPKIError")]
    WebPKIError(#[source] webpki::Error),
    #[error("SerdeJSONError: {0}")]
    SerdeJSONError(serde_json::Error),
    #[error("Base64Error")]
    Base64Error(#[from] base64::DecodeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

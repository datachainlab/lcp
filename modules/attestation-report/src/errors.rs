#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use std::string::String;

#[derive(Debug, thiserror::Error)]
pub enum AttestationReportError {
    #[error("InvalidAttestationReport: {0}")]
    InvalidAttestationReportError(String),
    #[error("UnexpectedAttestationReportVersionError: expected={0} actual={1}")]
    UnexpectedAttestationReportVersionError(String, String),
    #[error("InvalidQuoteStatusError: {0}")]
    InvalidQuoteStatusError(String),
    #[error("InvalidReportDataError: {0}")]
    InvalidReportDataError(String),
    #[error("WebPKIError")]
    WebPKIError(#[source] webpki::Error),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

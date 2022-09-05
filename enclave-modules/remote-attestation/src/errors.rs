use sgx_types::sgx_status_t;
use std::string::String;

#[derive(thiserror::Error, Debug)]
pub enum RemoteAttestationError {
    #[error("TooOldReportTimestamp")]
    TooOldReportTimestampError(String),
    #[error("AttestationReportError")]
    AttestationReportError(#[from] attestation_report::AttestationReportError),
    #[error("SGXError: status={0} description={1}")]
    SGXError(sgx_status_t, String),
    #[error("TimeError")]
    TimeError(#[from] lcp_types::TimeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

use core::time::Duration;

#[derive(thiserror::Error, Debug)]
pub enum IBCClientError {
    #[error("ExpiredAVRError: {current_timestamp:?} {attestation_time:?} {client_state_key_expiration:?}")]
    ExpiredAVRError {
        current_timestamp: lcp_types::Time,
        attestation_time: lcp_types::Time,
        client_state_key_expiration: Duration,
    },
    #[error("MrenclaveMismatchError: {0:?} {1:?}")]
    MrenclaveMismatchError(Vec<u8>, Vec<u8>),
    #[error("AttestationReportError")]
    AttestationReportError(#[from] attestation_report::AttestationReportError),
    #[error("TimeError")]
    TimeError(#[from] lcp_types::TimeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

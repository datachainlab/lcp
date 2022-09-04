use core::time::Duration;

#[derive(thiserror::Error, Debug)]
pub enum IBCClientError {
    #[error("ExpiredAVRError: {current_timestamp:?} {quote_timestamp:?} {client_state_key_expiration:?}")]
    ExpiredAVRError {
        current_timestamp: lcp_types::Time,
        quote_timestamp: lcp_types::Time,
        client_state_key_expiration: Duration,
    },
    #[error("MrenclaveMismatchError: {0:?} {1:?}")]
    MrenclaveMismatchError(Vec<u8>, Vec<u8>),
    #[error("AttestationReportError")]
    AttestationReportError(#[from] attestation_report::AttestationReportError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

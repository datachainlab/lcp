#[derive(thiserror::Error, Debug)]
pub enum IBCClientError {
    #[error("ExpiredAVRError: {0} {1}")]
    ExpiredAVRError(u128, u128),
    #[error("MrenclaveMismatchError: {0:?} {1:?}")]
    MrenclaveMismatchError(Vec<u8>, Vec<u8>),
    #[error("AttestationReportError")]
    AttestationReportError(#[from] attestation_report::AttestationReportError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

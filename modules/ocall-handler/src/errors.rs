use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(thiserror::Error, Debug)]
pub enum HandlerError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

impl From<sgx_status_t> for HandlerError {
    fn from(s: sgx_status_t) -> Self {
        HandlerError::SGXError(s)
    }
}

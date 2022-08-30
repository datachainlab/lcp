use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

use derive_more::Display;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(thiserror::Error, Debug, Display)]
pub enum RuntimeError {
    SGXError(sgx_status_t),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

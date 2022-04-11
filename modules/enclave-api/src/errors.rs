use bincode::Error as BincodeError;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, EnclaveAPIError>;

#[derive(thiserror::Error, Debug)]
pub enum EnclaveAPIError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),
    #[error("BincodeError: {0}")]
    BincodeError(BincodeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

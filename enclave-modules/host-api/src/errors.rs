use bincode::Error as BincodeError;
use sgx_types::sgx_status_t;
use std::string::String;

pub type Result<T> = std::result::Result<T, HostAPIError>;

#[derive(thiserror::Error, Debug)]
pub enum HostAPIError {
    #[error("SGXError: status={0}")]
    SGXError(sgx_status_t),
    #[error("CommandError: status={0} description={1}")]
    CommandError(sgx_status_t, String),
    #[error("BincodeError")]
    BincodeError(#[from] BincodeError),
}

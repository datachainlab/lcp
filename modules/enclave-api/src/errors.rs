use bincode::Error as BincodeError;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, EnclaveAPIError>;

#[derive(thiserror::Error, Debug)]
pub enum EnclaveAPIError {
    #[error("InvalidArgumentError: {0}")]
    InvalidArgumentError(String),
    #[error("SGXError: status={0}")]
    SGXError(sgx_status_t),
    #[error("CommandError: status={0} description={1}")]
    CommandError(sgx_status_t, String),
    #[error("BincodeError")]
    BincodeError(#[from] BincodeError),
    #[error("EnclaveCommandError")]
    EnclaveCommandError(#[from] ecall_commands::EnclaveCommandError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

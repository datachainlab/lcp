use bincode::Error as BincodeError;
use derive_more::Display;
use enclave_crypto::CryptoError;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(thiserror::Error, Debug, Display)]
pub enum StoreError {
    CryptoError(CryptoError),
    BincodeError(BincodeError),
    SGXError(sgx_status_t),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("CryptoError")]
    CryptoError(#[from] crypto::CryptoError),
    #[error("BincodeError")]
    BincodeError(#[from] bincode::Error),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

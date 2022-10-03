#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(thiserror::Error, Debug)]
pub enum HandlerError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),
    #[error("StoreError")]
    StoreError(#[from] store::StoreError),
    #[error("CryptoError")]
    CryptoError(#[from] crypto::CryptoError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

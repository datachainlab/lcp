use crypto::CryptoError;
use derive_more::Display;
use sgx_types::sgx_status_t;
use store::StoreError;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(thiserror::Error, Display, Debug)]
pub enum HandlerError {
    SGXError(sgx_status_t),
    StoreError(StoreError),
    CryptoError(CryptoError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

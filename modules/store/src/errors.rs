#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use bincode::Error as BincodeError;
use crypto::CryptoError;
use derive_more::Display;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(thiserror::Error, Debug, Display)]
pub enum StoreError {
    CryptoError(CryptoError),
    BincodeError(BincodeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

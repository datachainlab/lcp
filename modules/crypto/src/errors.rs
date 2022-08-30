#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use sgx_types::sgx_status_t;
use std::string::String;

#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),

    #[error("FailedSeal: {1}")]
    FailedSeal(#[source] std::io::Error, String),
    #[error("FailedUnseal: {1}")]
    FailedUnseal(#[source] std::io::Error, String),

    /// An error derived from secp256k1 error
    #[error("Secp256k1Error")]
    Secp256k1Error(#[from] secp256k1::Error),

    /// An error related to signature verification
    #[error("VerificationError: {0}")]
    VerificationError(String),

    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

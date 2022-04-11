#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use derive_more::Display;
use ibc::core::ics24_host::path::PathError;
use rlp::DecoderError;

// NOTE: this is a workaround to avoid importing enclave_crypto when the sgx feature is not enabled
// we should remove this after enclave_crypto crate supports non-sgx in the future.

#[cfg(feature = "sgx")]
#[derive(thiserror::Error, Display, Debug)]
pub enum CommitmentError {
    CryptoError(enclave_crypto::CryptoError),
    ICS24PathError(PathError),
    RLPDecoderError(DecoderError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[cfg(not(feature = "sgx"))]
#[derive(thiserror::Error, Display, Debug)]
pub enum CommitmentError {
    ICS24PathError(PathError),
    RLPDecoderError(DecoderError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

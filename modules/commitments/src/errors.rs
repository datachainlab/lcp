#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use derive_more::Display;
use ibc::core::ics24_host::path::PathError;
use rlp::DecoderError;

#[derive(thiserror::Error, Display, Debug)]
pub enum CommitmentError {
    CryptoError(crypto::CryptoError),
    ICS24PathError(PathError),
    RLPDecoderError(DecoderError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

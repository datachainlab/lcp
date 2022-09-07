#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum CommitmentError {
    #[error("CryptoError")]
    CryptoError(#[from] crypto::CryptoError),
    #[error("ICS23Error: {0}")]
    ICS23Error(ibc::core::ics23_commitment::error::Error),
    #[error("ICS24PathError: {0}")]
    ICS24PathError(ibc::core::ics24_host::path::PathError),
    #[error("RLPDecoderError: {0}")]
    RLPDecoderError(rlp::DecoderError),
    #[error("TypeError")]
    TypeError(#[from] lcp_types::TypeError),
    #[error("TimeError")]
    TimeError(#[from] lcp_types::TimeError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

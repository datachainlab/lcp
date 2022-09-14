#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use std::string::String;

#[derive(thiserror::Error, Debug)]
pub enum EnclaveCommandError {
    #[error("InvalidArgumentError: {0}")]
    InvalidArgumentError(String),
    #[error("ICS24ValidationError: {0}")]
    ICS24ValidationError(ibc::core::ics24_host::error::ValidationError),
}

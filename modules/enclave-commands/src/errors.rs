#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use std::string::String;

#[derive(thiserror::Error, Debug)]
pub enum EnclaveCommandError {
    #[error("InvalidArgumentError: {0}")]
    InvalidArgumentError(String),
    #[error("ICS03Error: {0}")]
    ICS03Error(ibc::core::ics03_connection::error::Error),
    #[error("ICS04Error: {0}")]
    ICS04Error(ibc::core::ics04_channel::error::Error),
    #[error("ICS24ValidationError: {0}")]
    ICS24ValidationError(ibc::core::ics24_host::error::ValidationError),
}

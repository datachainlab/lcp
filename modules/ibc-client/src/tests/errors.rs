#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics03_connection::error::Error as ICS03Error;
use ibc::core::ics04_channel::error::Error as ICS04Error;
use ibc::core::ics23_commitment::error::Error as ICS23Error;
use ibc::proofs::ProofError;
use light_client::{LightClientError, LightClientInstanceError};
use std::boxed::Box;
use std::string::String;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum LCPLCError {
    #[error("unexpected client type: {0}")]
    UnexpectedClientTypeError(String),
    #[error("ICS02Error: {0}")]
    ICS02Error(ICS02Error),
    #[error("ICS03Error: {0}")]
    ICS03Error(ICS03Error),
    #[error("ICS04Error: {0}")]
    ICS04Error(ICS04Error),
    #[error("ICS23Error: {0}")]
    ICS23Error(ICS23Error),
    #[error("IBCProofError: {0}")]
    IBCProofError(ProofError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

impl LightClientInstanceError for LCPLCError {}

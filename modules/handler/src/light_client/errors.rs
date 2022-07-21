#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::CommitmentError;
use derive_more::Display;
use ibc::core::ics02_client::error::Error as ICS02Error;
use light_client::LightClientError;

#[derive(thiserror::Error, Debug, Display)]
pub enum LightClientHandlerError {
    ICS02Error(ICS02Error),
    LightClientError(LightClientError),
    CommitmentError(CommitmentError),

    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

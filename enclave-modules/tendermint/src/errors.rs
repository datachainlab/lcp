use enclave_light_client::errors::{LightClientError, LightClientInstanceError};
use ibc::core::ics02_client::error::Error as ICS02Error;
use std::boxed::Box;
use std::string::String;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum TendermintError {
    #[error("unexpected client type: {0}")]
    UnexpectedClientTypeError(String),
    #[error("ICS02Error: {0}")]
    ICS02Error(ICS02Error),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

impl LightClientInstanceError for TendermintError {}

impl Into<LightClientError> for TendermintError {
    fn into(self) -> LightClientError {
        LightClientError::InstanceError(Arc::new(Box::new(self)))
    }
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::error::Error as ICS02Error;
use std::boxed::Box;
use std::fmt::{Debug, Display};
use std::string::String;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, LightClientError>;

#[derive(thiserror::Error, Debug)]
pub enum LightClientError {
    #[error("the type_url not found in the registry: {0}")]
    TypeUrlNotFoundError(String),
    #[error("the type_url already exists in the registry: {0}")]
    TypeUrlAlreadyExistsError(String),
    #[error("the registry is already sealed")]
    AlreadySealedError(),
    #[error("InstanceError: {0}")]
    InstanceError(Arc<Box<dyn LightClientInstanceError>>),
    #[error("ICS02Error: {0}")]
    ICS02Error(ICS02Error),
    #[error(transparent)]
    OtherError(anyhow::Error),
}

pub trait LightClientInstanceError: Display + Debug {}

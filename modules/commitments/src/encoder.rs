use crate::message::EmittedState;
use crate::prelude::*;
use crate::Error;
use alloy_sol_types::sol;
use lcp_types::{Any, Height};
use prost::Message;

pub trait EthABIEncoder {
    fn ethabi_encode(self) -> Vec<u8>;
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
}

sol! {
    struct EthABIHeight {
        uint64 revision_number;
        uint64 revision_height;
    }

    struct EthABIEmittedState {
        EthABIHeight height;
        bytes state;
    }
}

impl EthABIHeight {
    pub fn is_zero(&self) -> bool {
        self.revision_number == 0 && self.revision_height == 0
    }
}

impl From<EthABIHeight> for Height {
    fn from(value: EthABIHeight) -> Self {
        Self::new(value.revision_number, value.revision_height)
    }
}

impl From<EthABIHeight> for Option<Height> {
    fn from(value: EthABIHeight) -> Self {
        if value.is_zero() {
            None
        } else {
            Some(value.into())
        }
    }
}

impl From<Height> for EthABIHeight {
    fn from(value: Height) -> Self {
        Self {
            revision_number: value.revision_number(),
            revision_height: value.revision_height(),
        }
    }
}

impl From<Option<Height>> for EthABIHeight {
    fn from(value: Option<Height>) -> Self {
        value.unwrap_or_default().into()
    }
}

impl From<EmittedState> for EthABIEmittedState {
    fn from(value: EmittedState) -> Self {
        Self {
            height: value.0.into(),
            state: value.1.encode_to_vec().into(),
        }
    }
}

impl TryFrom<EthABIEmittedState> for EmittedState {
    type Error = Error;
    fn try_from(value: EthABIEmittedState) -> Result<Self, Self::Error> {
        Ok(Self(
            value.height.into(),
            Any::try_from(value.state.to_vec())?,
        ))
    }
}

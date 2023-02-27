use crate::prelude::*;
use flex_error::*;
use lcp_types::{ClientId, Height};

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        Commitment
        [commitments::Error]
        |_| { "Commitment error" },

        ClientTypeNotFound
        {
            client_id: ClientId
        }
        |e| {
            format_args!("client_type not found: client_id={}", e.client_id)
        },

        ClientStateNotFound
        {
            client_id: ClientId
        }
        |e| {
            format_args!("client_state not found: client_id={}", e.client_id)
        },

        ConsensusStateNotFound
        {
            client_id: ClientId,
            height: Height
        }
        |e| {
            format_args!("consensus_state not found: client_id={} height={}", e.client_id, e.height)
        },

        LightClientInstance
        [TraceError<Box<dyn LightClientInstanceError>>]
        |_| { "Light Client instance error" }
    }
}

pub trait LightClientInstanceError: core::fmt::Display + core::fmt::Debug + Sync + Send {}

impl<T: 'static + LightClientInstanceError> From<T> for Error {
    fn from(value: T) -> Self {
        Self::light_client_instance(Box::new(value))
    }
}

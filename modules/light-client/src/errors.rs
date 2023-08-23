use crate::prelude::*;
use crate::types::{ClientId, Height};
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        Commitment
        [crate::commitments::Error]
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

        LightClientSpecific
        [TraceError<Box<dyn LightClientSpecificError>>]
        |_| { "Light Client specific error" }
    }
}

/// Each Light Client's error type should implement this trait
pub trait LightClientSpecificError: core::fmt::Display + core::fmt::Debug + Sync + Send {}

impl<T: 'static + LightClientSpecificError> From<T> for Error {
    fn from(value: T) -> Self {
        Self::light_client_specific(Box::new(value))
    }
}

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    RegistryError {
        TypeUrlNotFound
        {
            type_url: String
        }
        |e| {
            format_args!("type_url not found: type_url={}", e.type_url)
        },

        TypeUrlAlreadyExists
        {
            type_url: String
        }
        |e| {
            format_args!("type_url already exists: type_url={}", e.type_url)
        },

        AlreadySealed
        |_| { "registry is already sealed" },
    }
}

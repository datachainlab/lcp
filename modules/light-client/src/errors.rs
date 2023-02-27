use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        Ics02
        [TraceError<ibc::core::ics02_client::error::ClientError>]
        |_|  { "ICS02 client error" },

        LightClientInstance
        [TraceError<Box<dyn LightClientInstanceError>>]
        |_| { "Light Client instance error" },

        Commitment
        [commitments::Error]
        |_| { "Commitment error" }
    }
}

pub trait LightClientInstanceError: core::fmt::Display + core::fmt::Debug + Sync + Send {}

impl<T: 'static + LightClientInstanceError> From<T> for Error {
    fn from(value: T) -> Self {
        Self::light_client_instance(Box::new(value))
    }
}

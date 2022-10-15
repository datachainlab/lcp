use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        Ics02
        [ibc::core::ics02_client::error::Error]
        |_| { "ICS02 client error" },

        LightClient
        [light_client::Error]
        |_| { "LightClient error" },

        LightClientRegistry
        [light_client_registry::Error]
        |_| { "LightClientRegistry error" },

        Commitment
        [commitments::Error]
        |_| { "Commitment error" },

        Ics24
        [ibc::core::ics24_host::error::ValidationError]
        |_| { "ICS24 host error" }
    }
}

impl From<commitments::Error> for Error {
    fn from(err: commitments::Error) -> Self {
        Error::commitment(err)
    }
}

impl From<light_client::Error> for Error {
    fn from(err: light_client::Error) -> Self {
        Error::light_client(err)
    }
}

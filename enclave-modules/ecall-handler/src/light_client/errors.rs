use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        SealedEnclaveKeyNotFound
        |_| { "Sealed EnclaveKey not found" },

        LightClient
        [light_client::Error]
        |_| { "LightClient error" },

        LightClientRegistry
        [light_client_registry::Error]
        |_| { "LightClientRegistry error" },

        Commitment
        [commitments::Error]
        |_| { "Commitment error" },

        LcpType
        {}
        [lcp_types::TypeError]
        |_| {"Type error"},
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

impl From<lcp_types::TypeError> for Error {
    fn from(err: lcp_types::TypeError) -> Self {
        Error::lcp_type(err)
    }
}

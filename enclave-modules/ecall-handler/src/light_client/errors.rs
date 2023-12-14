use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        InvalidArgument
        {
            descr: String
        }
        |e| {
            format_args!("invalid argument: descr={}", e.descr)
        },

        SealedEnclaveKeyNotFound
        |_| { "Sealed EnclaveKey not found" },

        LightClient
        [light_client::Error]
        |_| { "LightClient error" },

        LightClientRegistry
        [light_client::RegistryError]
        |_| { "LightClientRegistry error" },

        Commitment
        [light_client::commitments::Error]
        |_| { "Commitment error" },

        Crypto
        [crypto::Error]
        |_| { "Crypto error" },

        LcpType
        {}
        [lcp_types::TypeError]
        |_| {"Type error"},
    }
}

impl From<light_client::commitments::Error> for Error {
    fn from(err: light_client::commitments::Error) -> Self {
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

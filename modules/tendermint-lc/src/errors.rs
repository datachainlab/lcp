use crate::prelude::*;
use flex_error::*;
use light_client::LightClientSpecificError;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        UnexpectedClientType {
            type_url: String
        }
        |e| {
            format_args!("unexpected client_type: type_url={}", e.type_url)
        },

        Commitment
        [light_client::commitments::Error]
        |_| { "Commitment error" },

        InvalidTimestamp
        |_| { "Invalid timestamp" },

        Time
        [light_client::types::TimeError]
        |_| { "Time error" },
    }
}

impl LightClientSpecificError for Error {}

impl From<light_client::commitments::Error> for Error {
    fn from(err: light_client::commitments::Error) -> Self {
        Error::commitment(err)
    }
}

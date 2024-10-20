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
            format_args!("unexpected client_type error: type_url={}", e.type_url)
        },

        IbcHostDecoding
        [TraceError<ibc_core_host_types::error::DecodingError>]
        |_| { "IBC host decoding error" },
    }
}

impl LightClientSpecificError for Error {}

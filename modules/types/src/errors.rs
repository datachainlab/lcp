use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    TypeError {
        HeightBytesConversion
            {
                bz: Vec<u8>,
            }
            |e| {
                format_args!("height bytes length must be 16, but got {:?}", e.bz)
            },
        InvalidClientIdFormat
            {
                client_id_str: String,
                reason: String,
            }
            |e| {
                format_args!("invalid clientId format: got={:?} reason={:?}", e.client_id_str, e.reason)
            },
        MrenclaveBytesConversion
            {
                bz: Vec<u8>,
            }
            |e| {
                format_args!("mrenclave: bytes length must be 32, but got {:?}", e.bz)
            },
        HexParseError
            [TraceError<hex::FromHexError>]
            |_| { "hex parse error" },
    }
}

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    TimeError {
        Tendermint
            [tendermint::Error]
            |_| {
                "tendermint error"
            },
        TryFromIntError
            [TraceError<core::num::TryFromIntError>]
            |_| {"TryFromIntError"}
    }
}

impl From<hex::FromHexError> for TypeError {
    fn from(value: hex::FromHexError) -> Self {
        Self::hex_parse_error(value)
    }
}

impl From<core::num::TryFromIntError> for TimeError {
    fn from(value: core::num::TryFromIntError) -> Self {
        Self::try_from_int_error(value)
    }
}

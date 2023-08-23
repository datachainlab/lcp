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
        HeightConversion
            {
                height: String,
            }
            |e| {
                format_args!("height conversion error: {}", e.height)
            },
        InvalidHeightResult
            |_| {
                "height cannot end up zero or negative"
            },
        ClientIdContainSeparator
            { id: String }
            |e| {
                format_args!("identifier `{}` cannot contain separator '/'", e.id)
            },
        ClientIdInvalidLength
            {
                id: String,
                length: usize,
                min: usize,
                max: usize,
            }
            |e| {
                format_args!("identifier `{}` has invalid length `{}` must be between `{}`-`{}` characters", e.id, e.length, e.min, e.max)
            },
        ClientIdInvalidCharacter
            { id: String }
            |e| {
                format_args!("identifier `{}` must only contain alphanumeric characters or `.`, `_`, `+`, `-`, `#`, - `[`, `]`, `<`, `>`", e.id)
            },
        ClientIdEmpty
            |_| {
                "identifier cannot be empty"
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

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
                "height cannot end up negative or overflow"
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
        ClientIdInvalidFormat
            { id: String }
            |e| {
                format_args!("identifier `{}` must be in the format `{{client_type}}-{{counter}}`", e.id)
            },
        ClientIdInvalidClientType
            { id: String, client_type: String }
            |e| {
                format_args!("identifier `{}` must have client type `{}`", e.id, e.client_type)
            },
        ClientIdInvalidCounter
            { id: String }
            |e| {
                format_args!("identifier `{}` must have a valid counter", e.id)
            },
        ClientIdInvalidCounterParseIntError
            { id: String, e: core::num::ParseIntError }
            |e| {
                format_args!("identifier `{}` counter parse error: {}", e.id, e.e)
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
        ProtoError
            [TraceError<lcp_proto::protobuf::Error>]
            |_| { "proto error" },
    }
}

impl From<hex::FromHexError> for TypeError {
    fn from(value: hex::FromHexError) -> Self {
        Self::hex_parse_error(value)
    }
}

impl From<lcp_proto::protobuf::Error> for TypeError {
    fn from(value: lcp_proto::protobuf::Error) -> Self {
        Self::proto_error(value)
    }
}

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    TimeError {
        InvalidDate
            |_| {
                "invalid date"
            },
        DurationOutOfRange
            |_| { format_args!("duration value out of range") },
        ComponentRange
            [TraceError<time::error::ComponentRange>]
            |e| { format_args!("{}", e) },
        TryFromIntError
            [TraceError<core::num::TryFromIntError>]
            |_| {"TryFromIntError"}
    }
}

impl From<core::num::TryFromIntError> for TimeError {
    fn from(value: core::num::TryFromIntError) -> Self {
        Self::try_from_int_error(value)
    }
}

use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    InputValidationError {
        InvalidArgument {
            descr: String
        }
        |e| {
            format_args!("invalid argument: descr={}", e.descr)
        },
        LcpType
        {}
        [lcp_types::TypeError]
        |_| { "Type error" },
        Crypto
        {}
        [crypto::Error]
        |_| { "Crypto error" }
    }
}

impl From<lcp_types::TypeError> for InputValidationError {
    fn from(err: lcp_types::TypeError) -> Self {
        InputValidationError::lcp_type(err)
    }
}

impl From<crypto::Error> for InputValidationError {
    fn from(value: crypto::Error) -> Self {
        InputValidationError::crypto(value)
    }
}

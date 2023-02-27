use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        InvalidArgument {
            descr: String
        }
        |e| {
            format_args!("invalid argument: descr={}", e.descr)
        },
        LcpType
        {}
        [lcp_types::TypeError]
        |_| { "Type error" }
    }
}

impl From<lcp_types::TypeError> for Error {
    fn from(err: lcp_types::TypeError) -> Self {
        Error::lcp_type(err)
    }
}

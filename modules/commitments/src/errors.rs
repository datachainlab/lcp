use crate::prelude::*;
use crate::STATE_ID_SIZE;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        StringFromUtf8
        {}
        [TraceError<alloc::string::FromUtf8Error>]
        |_| {"StringFromUtf8"},

        EthAbiDecode
        {
            descr: String
        }
        |e| {
            format_args!("ethabi decode error: descr={}", e.descr)
        },

        InvalidAbi
        {
            descr: String
        }
        |e| {
            format_args!("invalid abi: descr={}", e.descr)
        },

        InvalidStateIdLength
        {
            actual: usize
        }
        |e| {
            format_args!("invalid stateID length: expected={} actual={}", STATE_ID_SIZE, e.actual)
        },

        InvalidOptionalBytesLength
        {
            expected: usize,
            actual: usize
        }
        |e| {
            format_args!("invalid bytes length: expected=0or{} actual={}", e.expected, e.actual)
        },

        UnexpectedCommitmentType
        {
            expected: u16,
            actual: u16
        }
        |e| {
            format_args!("unexpected commitment type: expected={} actual={}", e.expected, e.actual)
        },

        LcpType
        {}
        [lcp_types::TypeError]
        |_| {"Type"},

        LcpTime
        [lcp_types::TimeError]
        |_| {"Time"},

        Crypto
        [crypto::Error]
        |_| {"crypto error"},
    }
}

impl From<alloc::string::FromUtf8Error> for Error {
    fn from(err: alloc::string::FromUtf8Error) -> Self {
        Error::string_from_utf8(err)
    }
}

impl From<lcp_types::TypeError> for Error {
    fn from(err: lcp_types::TypeError) -> Self {
        Error::lcp_type(err)
    }
}

impl From<lcp_types::TimeError> for Error {
    fn from(err: lcp_types::TimeError) -> Self {
        Error::lcp_time(err)
    }
}

impl From<ethabi::Error> for Error {
    fn from(value: ethabi::Error) -> Self {
        Error::eth_abi_decode(format!("{:?}", value))
    }
}

impl From<crypto::Error> for Error {
    fn from(value: crypto::Error) -> Self {
        Error::crypto(value)
    }
}

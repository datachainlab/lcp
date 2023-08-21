use crate::prelude::*;
use crate::STATE_ID_SIZE;
use flex_error::*;
use lcp_types::Time;

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

        InvalidCommitmentHeader
        {
            descr: String
        }
        |e| {
            format_args!("invalid commitment header: descr={}", e.descr)
        },

        InvalidCommitmentContextHeader
        {
            descr: String
        }
        |e| {
            format_args!("invalid commitment context header: descr={}", e.descr)
        },

        OutOfTrustingPeriod
        {
            current_timestamp: Time,
            trusting_period_end: Time
        }
        |e| {
            format_args!("out of trusting period: current_timestamp={} trusting_period_end={}", e.current_timestamp, e.trusting_period_end)
        },

        HeaderFromFuture
        {
            current_timestamp: Time,
            header_timestamp: Time
        }
        |e| {
            format_args!("header is coming from future: current_timestamp={} header_timestamp={}", e.current_timestamp, e.header_timestamp)
        },

        NotTruncatedTimestamp
        {
            timestamp_nanos: u128
        }
        |e| {
            format_args!("not truncated timestamp: timestamp_nanos={}", e.timestamp_nanos)
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

        TryFromIntError
        [TraceError<core::num::TryFromIntError>]
        |_| {"TryFromIntError"}
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

impl From<core::num::TryFromIntError> for Error {
    fn from(value: core::num::TryFromIntError) -> Self {
        Error::try_from_int_error(value)
    }
}

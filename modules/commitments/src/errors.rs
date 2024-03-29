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
        [TraceError<alloy_sol_types::Error>]
        |_| {"ethabi decode error"},

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

        UnexpectedMessageType
        {
            expected: u16,
            actual: u16
        }
        |e| {
            format_args!("unexpected message type: expected={} actual={}", e.expected, e.actual)
        },

        InvalidMessageHeader
        {
            descr: String
        }
        |e| {
            format_args!("invalid message header: descr={}", e.descr)
        },

        InvalidValidationContextHeader
        {
            descr: String
        }
        |e| {
            format_args!("invalid validation context header: descr={}", e.descr)
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

        MessageAggregationFailed
        {
            descr: String
        }
        |e| {
            format_args!("message aggregation failed: descr={}", e.descr)
        },

        ContextAggregationFailed
        {
            descr: String
        }
        |e| {
            format_args!("context aggregation failed: descr={}", e.descr)
        },

        EmptyPath
        {}
        |_| {"empty path"},

        ZeroHeight
        {}
        |_| {"zero height"},

        ZeroStateId
        {}
        |_| {"zero stateID"},

        InvalidPrevStateAndHeight
        {}
        |_| {"prev_height and prev_state_id must be both None or both Some"},

        EmptyPrevStates
        {}
        |_| {"empty prev_states in misbehaviour message"},

        ProtoDecodeError
        [TraceError<prost::DecodeError>]
        |_| {"proto decode error"},

        LcpType
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

impl From<alloy_sol_types::Error> for Error {
    fn from(value: alloy_sol_types::Error) -> Self {
        Error::eth_abi_decode(value)
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

use crate::prelude::*;
use crate::STATE_ID_SIZE;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        RlpDecode
        [TraceError<rlp::DecoderError>]
        |_| {"RLP decode error"},

        InvalidCommitmentFormat
        {
            descr: String
        }
        |e| {
            format_args!("invalid commitment format: descr={}", e.descr)
        },

        InvalidStateIdLength
        {
            actual: usize
        }
        |e| {
            format_args!("invalid stateID length: expected={} actual={}", STATE_ID_SIZE, e.actual)
        },

        LcpType
        {}
        [lcp_types::TypeError]
        |_| {"Type"},

        LcpTime
        [lcp_types::TimeError]
        |_| {"Time"}
    }
}

#[cfg(feature = "prover")]
define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    ProverError {
        Crypto
        [crypto::Error]
        |_| {"crypto error"},
    }
}

impl From<rlp::DecoderError> for Error {
    fn from(err: rlp::DecoderError) -> Self {
        Error::rlp_decode(err)
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

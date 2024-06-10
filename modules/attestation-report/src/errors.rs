use crate::prelude::*;
use flex_error::*;
use lcp_types::Mrenclave;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        UnexpectedAttestationReportVersion
        {
            expected: i64,
            actual: i64
        }
        |e| {
            format_args!("unexpected attestation report version: expected={} actual={}", e.expected, e.actual)
        },

        UnexpectedReportDataVersion
        {
            expected: u8,
            actual: u8
        }
        |e| {
            format_args!("unexpected report data version: expected={} actual={}", e.expected, e.actual)
        },

        InvalidReportDataSize
        {
            size: usize
        }
        |e| {
            format_args!("invalid report data size: size must be >= 20, but got {}", e.size)
        },

        MrenclaveMismatch
        {
            expected: Mrenclave,
            actual: Mrenclave
        }
        |e| {
            format_args!("Mrenclave mismatch error: expected={} actual={}", e.expected, e.actual)
        },

        WebPki
        {
            descr: String
        }
        |e| {
            format_args!("WebPKI error: descr={}", e.descr)
        },

        SerdeJson
        [TraceError<serde_json::Error>]
        |_| { "serde_json error" },

        Base64
        [TraceError<base64::DecodeError>]
        |_| { "base64 error" },

        TimeError
        [lcp_types::TimeError]
        |_| { "Time error" },

        CryptoError
        [crypto::Error]
        |_| { "Crypto error" }
    }
}

impl From<crypto::Error> for Error {
    fn from(value: crypto::Error) -> Self {
        Self::crypto_error(value)
    }
}

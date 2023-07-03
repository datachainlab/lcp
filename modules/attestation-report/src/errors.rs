use crate::prelude::*;
use flex_error::*;

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

        InvalidReportDataSize
        {
            size: usize
        }
        |e| {
            format_args!("invalid report data size: size must be >= 20, but got {}", e.size)
        },

        MrenclaveMismatch
        {
            expected: [u8; 32],
            actual: [u8; 32]
        }
        |e| {
            format_args!("Mrenclave mismatch error: expected=0x{} actual=0x{}", hex::encode(e.expected), hex::encode(e.actual))
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

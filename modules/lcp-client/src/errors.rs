use crate::prelude::*;
use core::time::Duration;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        UnexpectedClientType {
            type_url: String
        }
        |e| {
            format_args!("unexpected client_type: type_url={}", e.type_url)
        },

        UnexpectedHeaderType {
            type_url: String
        }
        |e| {
            format_args!("unexpected header type: type_url={}", e.type_url)
        },

        UnexpectedQuoteBody
        |e| {
            "unexpected quote body"
        },

        ExpiredAvr {
            current_timestamp: light_client::types::Time,
            attestation_time: light_client::types::Time,
            client_state_key_expiration: Duration
        }
        |e| {
            format_args!("Expired AVR: current_timestamp= {:?} attestation_time={:?} client_state_key_expiration={:?}", e.current_timestamp, e.attestation_time, e.client_state_key_expiration)
        },

        MrenclaveMismatch {
            expected: Vec<u8>,
            actual: Vec<u8>
        }
        |e| {
            format_args!("Mrenclave mismatch: expected={:?} actual={:?}", e.expected, e.actual)
        },

        InvalidZkdcapVerifierInfo {
            bytes: Vec<u8>
        }
        |e| {
            format_args!("Invalid zkdcap_verifier_info: bytes={:?}", e.bytes)
        },

        AttestationReport
        [attestation_report::Error]
        |_| { "Attestation report error" },

        Time
        [light_client::types::TimeError]
        |_| { "Time error" },

        CryptoError
        [crypto::Error]
        |_| { "Crypto error" },

        LightClientError
        [light_client::Error]
        |_| { "Light Client error" },

        CommitmentProof
        [light_client::commitments::Error]
        |_| { "Commitment proof error" },

        Zkvm
        [zkvm::Error]
        |_| { "Zkvm error" },

        IbcProto
        [TraceError<light_client::types::proto::protobuf::Error>]
        |_| { "IBCProto error" },

        StringFromUtf8Error
        [TraceError<alloc::string::FromUtf8Error>]
        |_| { "FromUtf8 error" },
    }
}

impl From<attestation_report::Error> for Error {
    fn from(err: attestation_report::Error) -> Self {
        Error::attestation_report(err)
    }
}

impl From<light_client::types::TimeError> for Error {
    fn from(err: light_client::types::TimeError) -> Self {
        Error::time(err)
    }
}

impl From<crypto::Error> for Error {
    fn from(value: crypto::Error) -> Self {
        Self::crypto_error(value)
    }
}

impl From<light_client::Error> for Error {
    fn from(value: light_client::Error) -> Self {
        Self::light_client_error(value)
    }
}

impl From<light_client::commitments::Error> for Error {
    fn from(value: light_client::commitments::Error) -> Self {
        Self::commitment_proof(value)
    }
}

impl From<zkvm::Error> for Error {
    fn from(value: zkvm::Error) -> Self {
        Self::zkvm(value)
    }
}

impl From<alloc::string::FromUtf8Error> for Error {
    fn from(value: alloc::string::FromUtf8Error) -> Self {
        Self::string_from_utf8_error(value)
    }
}

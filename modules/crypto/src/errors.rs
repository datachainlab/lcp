use crate::prelude::*;
use crate::EnclavePublicKey;
use flex_error::*;
use sgx_types::sgx_status_t;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        SgxError
        {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("SGX error: status={:?} descr={}", e.status, e.descr)
        },

        FailedSeal
        {
            descr: String
        }
        |e| {
            format_args!("failed to seal: descr={}", e.descr)
        },

        FailedUnseal
        {
            descr: String,
        }
        |e| {
            format_args!("failed to unseal: descr={}", e.descr)
        },

        InvalidSealedEnclaveKey
        {
            descr: String,
        }
        |e| {
            format_args!("invalid sealed Enclave Key: descr={}", e.descr)
        },

        InvalidAddressLength
        {
            length: usize,
        }
        |e| {
            format_args!("invalid address length: expected=20 actual={}", e.length)
        },

        InvalidSignatureLength
        {
            length: usize,
        }
        |e| {
            format_args!("invalid signature length: expected=65 actual={}", e.length)
        },

        InsufficientSecretKeySize
        {
            path: String,
            expected: usize,
            actual: usize
        }
        |e| {
            format_args!("dramatic read from {} ended prematurely (n = {} < SECRET_KEY_SIZE = {})", e.path, e.actual, e.expected)
        },

        UnexpectedSigner
        {
            expected: EnclavePublicKey,
            actual: EnclavePublicKey
        }
        |e| {
            format_args!("unexpected signer: expected={:?} actual={:?}", e.expected, e.actual)
        },

        NopSigner
        |_| { "nop signer doesn't support any operations" },

        Secp256k1
        [TraceError<libsecp256k1::Error>]
        |_| { "secp256k1 error" },

        HexParseError
        [TraceError<hex::FromHexError>]
        |_| { "hex parse error" },
    }
}

impl From<libsecp256k1::Error> for Error {
    fn from(value: libsecp256k1::Error) -> Self {
        Self::secp256k1(value)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(value: hex::FromHexError) -> Self {
        Self::hex_parse_error(value)
    }
}

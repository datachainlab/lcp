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
        }
        |e| {
            format_args!("SGX error: {:?}", e.status)
        },

        FailedSeal
        {
            err: String,
            path: String,
        }
        |e| {
            format_args!("failed to seal: path={} err={}", e.path, e.err)
        },

        FailedUnseal
        {
            err: String,
            path: String,
        }
        |e| {
            format_args!("failed to unseal: path={} err={}", e.path, e.err)
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

        Secp256k1
        [TraceError<secp256k1::Error>]
        |_| { "secp256k1 error" },

        UnexpectedSigner
        {
            expected: EnclavePublicKey,
            actual: EnclavePublicKey
        }
        |e| {
            format_args!("unexpected signer: expected={:?} actual={:?}", e.expected, e.actual)
        }
    }
}

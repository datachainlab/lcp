use crate::prelude::*;
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
            format_args!("SGX error: status={:?}, descr={}", e.status, e.descr)
        },

        EnclaveKeyNotFound
        |_| { "Enclave Key not found" },

        Crypto
        [crypto::Error]
        |_| { "Crypto error" },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" },

        EcallCommand
        [ecall_commands::InputValidationError]
        |_| { "EcallCommand input validation error" },

        Time
        [lcp_types::TimeError]
        |_| { "Time error" }
    }
}

impl From<attestation_report::Error> for Error {
    fn from(err: attestation_report::Error) -> Self {
        Error::attestation_report(err)
    }
}

impl From<crypto::Error> for Error {
    fn from(err: crypto::Error) -> Self {
        Error::crypto(err)
    }
}

impl From<ecall_commands::InputValidationError> for Error {
    fn from(err: ecall_commands::InputValidationError) -> Self {
        Error::ecall_command(err)
    }
}

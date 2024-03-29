use flex_error::*;
use sgx_types::sgx_status_t;

pub type Result<T> = std::result::Result<T, Error>;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        InvalidArgument {
            descr: String
        }
        |e| {
            format_args!("invalid argument: descr={}", e.descr)
        },

        SgxError
        {
            status: sgx_status_t
        }
        |e| {
            format_args!("SGX error: {:?}", e.status)
        },

        BincodeEncode
        [TraceError<bincode::error::EncodeError>]
        |_| { "bincode encode error" },

        BincodeDecode
        [TraceError<bincode::error::DecodeError>]
        |_| { "bincode decode error" },

        Command {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("Command error: status={:?} descr={}", e.status, e.descr)
        },

        EcallCommand
        [ecall_commands::InputValidationError]
        |_| { "ECallCommand input validation error" },

        Store
        [store::Error]
        |_| { "Store error" },

        KeyManager
        [keymanager::Error]
        |_| { "KeyManager error" },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" },

        Commitments
        [commitments::Error]
        |_| { "Commitments error" },
    }
}

impl From<sgx_status_t> for Error {
    fn from(value: sgx_status_t) -> Self {
        Self::sgx_error(value)
    }
}

impl From<ecall_commands::InputValidationError> for Error {
    fn from(err: ecall_commands::InputValidationError) -> Self {
        Error::ecall_command(err)
    }
}

impl From<store::Error> for Error {
    fn from(err: store::Error) -> Self {
        Error::store(err)
    }
}

impl From<keymanager::Error> for Error {
    fn from(err: keymanager::Error) -> Self {
        Error::key_manager(err)
    }
}

impl From<attestation_report::Error> for Error {
    fn from(err: attestation_report::Error) -> Self {
        Error::attestation_report(err)
    }
}

impl From<commitments::Error> for Error {
    fn from(err: commitments::Error) -> Self {
        Error::commitments(err)
    }
}

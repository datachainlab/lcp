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
        [ecall_commands::Error]
        |_| { "ECallCommand error" },

        Store
        [store::Error]
        |_| { "Store error" },

        KeyManager
        [keymanager::Error]
        |_| { "KeyManager error" },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" }
    }
}

impl From<ecall_commands::Error> for Error {
    fn from(err: ecall_commands::Error) -> Self {
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

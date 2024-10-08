use flex_error::*;
use sgx_types::sgx_status_t;

pub type Result<T> = core::result::Result<T, Error>;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        SgxError {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("SGX error: status={:?} descr={}", e.status, e.descr)
        },

        Store
        [host_environment::store::Error]
        |_| { "Store error" },

        Io
        [TraceError<std::io::Error>]
        |_| { "I/O error" },
    }
}

impl From<host_environment::store::Error> for Error {
    fn from(err: host_environment::store::Error) -> Self {
        Error::store(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::io(err)
    }
}

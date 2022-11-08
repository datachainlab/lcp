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
        [store::Error]
        |_| { "Store error" }
    }
}

impl From<store::Error> for Error {
    fn from(err: store::Error) -> Self {
        Error::store(err)
    }
}

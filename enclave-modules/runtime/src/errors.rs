use crate::prelude::*;
use flex_error::define_error;
use sgx_types::sgx_status_t;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        SgxError {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("SGX error: status={:?} descr={}", e.status, e.descr)
        }
    }
}

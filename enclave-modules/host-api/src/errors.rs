use crate::prelude::*;
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
            format_args!("SGX error: status={:?}", e.status)
        },
        Command
        {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("Command error: status={:?} description={}", e.status, e.descr)
        },
        Bincode
        [TraceError<bincode::Error>]
        |_| { "bincode error" }
    }
}

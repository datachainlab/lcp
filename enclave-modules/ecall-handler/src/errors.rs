use crate::prelude::*;
use flex_error::*;
use sgx_types::sgx_status_t;

pub type Result<T> = core::result::Result<T, Error>;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        SgxError
        {
            status: sgx_status_t,
        }
        |e| {
            format_args!("SGX error: {:?}", e.status)
        },

        EnclaveKeyNotFound
        |_| { "Enclave Key not found" },

        Store
        [store::Error]
        |_| { "Store error" },

        EnclaveManageCommand
        [crate::enclave_manage::Error]
        |_| { "EnclaveManageCommand error" },

        LightClientCommand
        [crate::light_client::Error]
        |_| { "LightClientCommand error" }
    }
}

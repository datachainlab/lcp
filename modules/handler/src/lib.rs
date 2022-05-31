#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use log_sgx as log;
    pub use thiserror_sgx as thiserror;
}
#[cfg(feature = "sgx")]
mod enclave_manage;
pub use errors::{HandlerError, Result};
mod errors;
mod light_client;
pub mod router;

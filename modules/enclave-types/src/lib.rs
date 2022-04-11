#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

pub use types::*;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}
// A trick to suppress an IDE error
#[cfg(not(feature = "sgx"))]
pub use anyhow;
#[cfg(feature = "sgx")]
pub use anyhow_sgx as anyhow;

pub mod commands;
pub mod types;

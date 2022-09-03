#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
#[macro_use]
extern crate alloc;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use libsecp256k1_sgx as secp256k1;
    pub use log_sgx as log;
    pub use thiserror_sgx as thiserror;
}

pub mod client_def;
pub mod client_state;
pub mod consensus_state;
mod errors;
pub mod header;
mod report;
#[cfg(test)]
mod tests;

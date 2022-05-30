#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use bincode_sgx as bincode;
    pub use log_sgx as log;
}

pub use client::{LightClientKeeper, LightClientReader};
pub use context::Context;

mod client;
mod context;

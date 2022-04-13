#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use libsecp256k1_sgx as libsecp256k1;
    pub use sgx_tstd as std;
}

pub mod client_def;
pub mod client_state;
pub mod consensus_state;
pub mod header;
mod public_key;

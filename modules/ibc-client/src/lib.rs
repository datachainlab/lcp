#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use libsecp256k1_sgx as secp256k1;
    pub use sgx_tstd as std;
}

pub mod client_def;
pub mod client_state;
pub mod consensus_state;
mod crypto;
pub mod header;

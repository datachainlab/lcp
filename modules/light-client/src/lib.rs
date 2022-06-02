#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}

pub use client::{
    CreateClientResult, LightClient, LightClientKeeper, LightClientReader, StateVerificationResult,
    UpdateClientResult,
};
pub use errors::{LightClientError, LightClientInstanceError, Result};
pub use registry::{LightClientRegistry, LightClientSource};

mod client;
mod errors;
mod registry;

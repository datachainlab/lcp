#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

pub use client::{CreateClientResult, LightClient, StateVerificationResult, UpdateClientResult};
pub use errors::{LightClientError, LightClientInstanceError, Result};
pub use registry::{LightClientRegistry, LightClientSource};

mod client;
mod errors;
mod registry;

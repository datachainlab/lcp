#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

pub use errors::HostAPIError;

pub mod api;
mod errors;
mod ffi;
pub mod remote_attestation;

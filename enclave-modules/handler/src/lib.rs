#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

pub use errors::{HandlerError, Result};

pub mod context;
mod enclave_manage;
mod errors;
mod light_client;
pub mod router;

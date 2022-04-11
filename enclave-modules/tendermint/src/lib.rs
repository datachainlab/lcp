#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

pub use client::{register_implementations, TendermintLightClient};

mod client;
mod errors;

#[macro_use]
extern crate alloc;

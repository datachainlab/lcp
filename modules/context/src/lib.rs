#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

pub use client::{LightClientKeeper, LightClientReader};

mod client;

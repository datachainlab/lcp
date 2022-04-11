#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;
extern crate sgx_types;

pub mod macros;
pub mod pointers;
pub mod results;
pub mod storage;

#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;
extern crate sgx_types;

extern crate enclave_runtime;

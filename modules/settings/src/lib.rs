#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

use lazy_static::lazy_static;
use std::string::{String, ToString};

lazy_static! {
    pub static ref ENCLAVE_KEY_SEALING_PATH: String = "ek_sealing".to_string();
    pub static ref ENDORSED_ATTESTATION_PATH: String = "endorsed_attestation".to_string();
    pub static ref COMMIT_ID_DIR: String = "commits".to_string();
    pub static ref LAST_COMMIT_SEQUENCE: String = "last_commit_seq".to_string();
}

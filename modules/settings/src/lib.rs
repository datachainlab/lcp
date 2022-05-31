#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

use lazy_static::lazy_static;
use sgx_types::sgx_status_t;
use std::string::{String, ToString};

lazy_static! {
    pub static ref ENCLAVE_KEY_SEALING_PATH: String = "ek_sealing".to_string();
    pub static ref ENDORSED_ATTESTATION_PATH: String = "endorsed_attestation".to_string();
    pub static ref COMMIT_ID_DIR: String = "commits".to_string();
    pub static ref LAST_COMMIT_SEQUENCE: String = "last_commit_seq".to_string();
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub enum SigningMethod {
    MRSIGNER,
    MRENCLAVE,
    NONE,
}

#[cfg(feature = "production")]
pub const SIGNING_METHOD: SigningMethod = SigningMethod::MRENCLAVE;

#[cfg(all(not(feature = "production"), not(feature = "test")))]
pub const SIGNING_METHOD: SigningMethod = SigningMethod::MRENCLAVE;

#[cfg(all(not(feature = "production"), feature = "test"))]
pub const SIGNING_METHOD: SigningMethod = SigningMethod::MRSIGNER;

#[cfg(feature = "production")]
pub static RT_ALLOWED_STATUS: &'static [sgx_status_t] = &[];

#[cfg(not(feature = "production"))]
pub static RT_ALLOWED_STATUS: &'static [sgx_status_t] = &[sgx_status_t::SGX_ERROR_UPDATE_NEEDED];

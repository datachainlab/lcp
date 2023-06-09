#![no_std]
use sgx_types::sgx_status_t;

pub static SEALED_ENCLAVE_KEY_PATH: &str = "ek_sealed";
pub static AVR_KEY_PATH: &str = "avr";

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
pub static RT_ALLOWED_STATUS: &[sgx_status_t] = &[];

#[cfg(not(feature = "production"))]
pub static RT_ALLOWED_STATUS: &[sgx_status_t] = &[sgx_status_t::SGX_ERROR_UPDATE_NEEDED];

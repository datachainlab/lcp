use sgx_types::sgx_status_t;

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

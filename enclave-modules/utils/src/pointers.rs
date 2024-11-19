use log::*;
use sgx_trts::trts::{rsgx_lfence, rsgx_sfence};
use sgx_types::*;

/// Validates a mutable pointer and its length.
///
/// Assumes that the `ptr` is a valid pointer of enclave outside memory.
pub fn validate_mut_ptr(ptr: *mut u8, ptr_len: usize) -> SgxResult<()> {
    if ptr.is_null() || ptr_len == 0 {
        warn!("Tried to access an empty pointer - ptr.is_null() || ptr_len == 0");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }
    rsgx_sfence();
    Ok(())
}

/// Validates a constant pointer and its length.
///
/// Assumes that the `ptr` is a valid pointer of enclave outside memory.
pub fn validate_const_ptr(ptr: *const u8, ptr_len: usize) -> SgxResult<()> {
    if ptr.is_null() || ptr_len == 0 {
        warn!("Tried to access an empty pointer - ptr.is_null() || ptr_len == 0");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }
    rsgx_lfence();
    Ok(())
}

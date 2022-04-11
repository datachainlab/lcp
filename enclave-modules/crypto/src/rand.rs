use sgx_rand::Rng;
use sgx_types::sgx_status_t;

pub fn fill_bytes(bytes: &mut [u8]) -> Result<(), sgx_status_t> {
    let mut os_rng = match sgx_rand::SgxRng::new() {
        Ok(r) => r,
        Err(_) => return Err(sgx_status_t::SGX_ERROR_UNEXPECTED),
    };
    os_rng.fill_bytes(bytes);
    Ok(())
}

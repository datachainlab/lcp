use crate::CryptoError as Error;
use sgx_rand::Rng;
use sgx_trts::trts::rsgx_read_rand;
use sgx_types::sgx_status_t;

pub fn rand_slice(rand: &mut [u8]) -> Result<(), Error> {
    rsgx_read_rand(rand).map_err(|_e| Error::RandomError {})
}

pub fn fill_bytes(bytes: &mut [u8]) -> Result<(), sgx_status_t> {
    let mut os_rng = match sgx_rand::SgxRng::new() {
        Ok(r) => r,
        Err(_) => return Err(sgx_status_t::SGX_ERROR_UNEXPECTED),
    };
    os_rng.fill_bytes(bytes);
    Ok(())
}

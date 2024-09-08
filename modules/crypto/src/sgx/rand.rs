use crate::errors::Error;
use sgx_trts::trts::rsgx_read_rand;
use sgx_types::sgx_status_t;

pub fn rand_slice(rand: &mut [u8]) -> Result<(), Error> {
    rsgx_read_rand(rand).map_err(Error::sgx_error)
}

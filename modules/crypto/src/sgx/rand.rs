use crate::errors::Error;
use crate::prelude::*;
use sgx_trts::trts::rsgx_read_rand;

pub fn rand_slice(rand: &mut [u8]) -> Result<(), Error> {
    rsgx_read_rand(rand).map_err(|e| Error::sgx_error(e, "failed to read random data".to_string()))
}

use crate::CryptoError as Error;
use sgx_trts::trts::rsgx_read_rand;

pub fn rand_slice(rand: &mut [u8]) -> Result<(), Error> {
    rsgx_read_rand(rand).map_err(|_e| Error::RandomError {})
}

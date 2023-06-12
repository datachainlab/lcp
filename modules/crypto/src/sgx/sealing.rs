use crate::prelude::*;
use crate::traits::SealedKey;
use crate::EnclaveKey;
use crate::Error;
use libsecp256k1::{util::SECRET_KEY_SIZE, SecretKey};
use sgx_tstd::{
    io::{Read, Write},
    sgxfs::SgxFile,
};

fn seal(data: &[u8; 32], filepath: &str) -> Result<(), Error> {
    let mut file = SgxFile::create(filepath)
        .map_err(|e| Error::failed_seal(e.to_string(), filepath.into()))?;
    file.write_all(data)
        .map_err(|e| Error::failed_seal(e.to_string(), filepath.into()))
}

fn unseal(filepath: &str) -> Result<SecretKey, Error> {
    let mut file = SgxFile::open(filepath)
        .map_err(|e| Error::failed_unseal(e.to_string(), filepath.into()))?;

    let mut buf = [0u8; SECRET_KEY_SIZE];
    let n = file
        .read(buf.as_mut())
        .map_err(|e| Error::failed_unseal(e.to_string(), filepath.into()))?;

    if n < SECRET_KEY_SIZE {
        return Err(Error::insufficient_secret_key_size(
            filepath.into(),
            SECRET_KEY_SIZE,
            n,
        ));
    }
    Ok(SecretKey::parse(&buf).unwrap())
}

impl SealedKey for EnclaveKey {
    fn seal(&self, filepath: &str) -> Result<(), Error> {
        // Files are automatically closed when they go out of scope.
        seal(&self.get_privkey(), filepath)
    }

    fn unseal(filepath: &str) -> Result<Self, Error> {
        let secret_key = unseal(filepath)?;
        Ok(Self { secret_key })
    }
}

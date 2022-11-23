use crate::errors::Error;
use crate::prelude::*;
use crate::traits::SealedKey;
use crate::EnclaveKey;
use settings::SEALED_ENCLAVE_KEY_PATH;
use sgx_tstd::path::PathBuf;

#[derive(Default)]
pub struct KeyManager {
    key_dir: String,
    enclave_key: Option<EnclaveKey>,
}

impl<'a> KeyManager {
    pub fn new(home: String) -> Self {
        let key_dir = PathBuf::from(home)
            .join(PathBuf::from(SEALED_ENCLAVE_KEY_PATH))
            .to_str()
            .unwrap()
            .to_string();
        let mut km = Self {
            key_dir,
            enclave_key: None,
        };
        let _ = km.load_enclave_key();
        km
    }

    pub fn load_enclave_key(&mut self) -> Result<(), Error> {
        let enclave_key = EnclaveKey::unseal(&self.key_dir)?;
        self.enclave_key = Some(enclave_key);
        Ok(())
    }

    pub fn create_enclave_key(&'a mut self) -> Result<&'a EnclaveKey, Error> {
        self.set_enclave_key(EnclaveKey::new()?)
    }

    pub fn set_enclave_key(&'a mut self, kp: EnclaveKey) -> Result<&'a EnclaveKey, Error> {
        kp.seal(&self.key_dir)?;
        self.enclave_key = Some(kp);
        Ok(self.enclave_key.as_ref().unwrap())
    }

    pub fn get_enclave_key(&'a self) -> Option<&'a EnclaveKey> {
        self.enclave_key.as_ref()
    }
}

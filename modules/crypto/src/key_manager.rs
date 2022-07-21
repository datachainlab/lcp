use crate::errors::CryptoError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{traits::SealedKey, EnclaveKey};
use log::*;
use settings::SEALED_ENCLAVE_KEY_PATH;
use std::{
    path::PathBuf,
    string::{String, ToString},
};

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
        match EnclaveKey::new() {
            Ok(key) => match self.set_enclave_key(key) {
                Ok(ek) => Ok(ek),
                Err(_) => Err(Error::KeyError),
            },
            Err(err) => Err(err),
        }
    }

    pub fn set_enclave_key(&'a mut self, kp: EnclaveKey) -> Result<&'a EnclaveKey, Error> {
        if let Err(e) = kp.seal(&self.key_dir) {
            error!("Error sealing registration key");
            return Err(e);
        }
        self.enclave_key = Some(kp);
        Ok(self.enclave_key.as_ref().unwrap())
    }

    pub fn get_enclave_key(&'a self) -> Option<&'a EnclaveKey> {
        self.enclave_key.as_ref()
    }
}

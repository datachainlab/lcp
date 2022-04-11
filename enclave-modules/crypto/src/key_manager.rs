use crate::errors::CryptoError as Error;
use crate::{traits::SealedKey, EnclaveKey};
use log::*;
use settings::ENCLAVE_KEY_SEALING_PATH;

#[derive(Default)]
pub struct KeyManager {
    enclave_key: Option<EnclaveKey>,
}

impl<'a> KeyManager {
    pub fn new() -> Self {
        let mut km = Self::default();
        let _ = km.load_enclave_key();
        km
    }

    pub fn load_enclave_key(&mut self) -> Result<(), Error> {
        let enclave_key = EnclaveKey::unseal(&ENCLAVE_KEY_SEALING_PATH)?;
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
        if let Err(e) = kp.seal(&ENCLAVE_KEY_SEALING_PATH) {
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

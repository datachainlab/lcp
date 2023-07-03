pub mod errors;
use crate::errors::Error;
use crypto::{Address, SealedEnclaveKey};
use std::path::PathBuf;

pub struct EnclaveKeyManager {
    key_dir: String,
}

// TODO implement this with files or rocksdb
impl EnclaveKeyManager {
    pub fn new(home_dir: &PathBuf) -> Self {
        Self { key_dir: todo!() }
    }

    pub fn load(&self, addr: Address) -> Result<SealedEnclaveKeyInfo, Error> {
        todo!()
    }

    pub fn save(&self, addr: Address, sealed_ek: SealedEnclaveKey) -> Result<(), Error> {
        todo!()
    }

    pub fn save_avr(&self, addr: Address, avr: String) -> Result<(), Error> {
        todo!()
    }

    pub fn list(&self) -> Result<SealedEnclaveKeyInfo, Error> {
        todo!()
    }

    pub fn prune(&self) -> Result<(), Error> {
        todo!()
    }
}

pub struct SealedEnclaveKeyInfo {
    pub address: Address,
    pub sealed_ek: SealedEnclaveKey,
    pub avr: String,
}

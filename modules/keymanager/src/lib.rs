pub mod errors;
use crate::errors::Error;
use crypto::{Address, SealedEnclaveKey};
use fslock::LockFile;
use log::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::{io::Write, path::PathBuf, time::SystemTime};
use tempfile::NamedTempFile;

pub static ENCLAVE_KEYS_DIR: &str = "keys";
pub static LOCK_FILE: &str = "keys.lock";

/**
 * Directory layout:
 * - key_<address> => SealedEnclaveKeyInfo as bytes
 * - keys.lock => LOCKFILE with pid
*/
pub struct EnclaveKeyManager {
    key_dir: PathBuf,
}

impl EnclaveKeyManager {
    pub fn new(home_dir: &PathBuf) -> Result<Self, Error> {
        let key_dir = home_dir.join(ENCLAVE_KEYS_DIR);

        if !key_dir.exists() {
            fs::create_dir_all(&key_dir)?;
            info!("created keys directory: {:?}", key_dir);
        }

        Ok(Self { key_dir })
    }

    pub fn load(&self, address: Address) -> Result<SealedEnclaveKeyInfo, Error> {
        self.read_file(self.key_path(address))
    }

    pub fn save(&self, address: Address, sealed_ek: SealedEnclaveKey) -> Result<(), Error> {
        let _lock = self.lock_blocking()?;
        let bz = serde_json::to_vec(&SealedEnclaveKeyInfo {
            address,
            sealed_ek,
            avr: Default::default(),
        })?;
        self.create_file(address, &bz)
    }

    pub fn save_avr(&self, address: Address, avr: String) -> Result<(), Error> {
        let _lock = self.lock_blocking()?;
        let mut ski = self.load(address)?;
        assert!(ski.avr.is_empty());
        ski.avr = avr;
        let bz = serde_json::to_vec(&ski)?;
        self.create_file(address, &bz)
    }

    pub fn list(&self) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let _lock = self.lock_blocking()?;
        let mut skis = Vec::new();
        for entry in fs::read_dir(&self.key_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("key_") {
                    skis.push(self.read_file(entry.path())?);
                }
            }
        }
        Ok(skis)
    }

    pub fn prune(&self, expired_at: SystemTime) -> Result<(), Error> {
        // let _lock = self.lock_blocking()?;
        todo!()
    }

    fn create_file(&self, address: Address, content: &[u8]) -> Result<(), Error> {
        let mut temp = NamedTempFile::new()?;
        temp.write_all(content)?;
        temp.flush()?;
        temp.persist(self.key_path(address))?;
        Ok(())
    }

    fn read_file(&self, key_file: PathBuf) -> Result<SealedEnclaveKeyInfo, Error> {
        let content = fs::read(key_file)?;
        Ok(serde_json::from_slice(&content)?)
    }

    fn lock_blocking(&self) -> Result<LockFile, Error> {
        let mut lock = LockFile::open(&self.key_dir.join(LOCK_FILE))?;
        lock.lock_with_pid()?;
        Ok(lock)
    }

    fn key_path(&self, address: Address) -> PathBuf {
        self.key_dir.join(format!("{}.key", address))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKeyInfo {
    pub address: Address,
    pub sealed_ek: SealedEnclaveKey,
    pub avr: String,
}

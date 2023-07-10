pub mod errors;
pub use crate::errors::Error;
use attestation_report::EndorsedAttestationVerificationReport;
use core::time::Duration;
use crypto::{Address, SealedEnclaveKey};
use fslock::LockFile;
use lcp_proto::lcp::service::enclave::v1::EnclaveKeyInfo as ProtoEnclaveKeyInfo;
use lcp_types::Time;
use log::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::{io::Write, path::PathBuf};
use tempfile::NamedTempFile;

pub static ENCLAVE_KEYS_DIR: &str = "keys";
pub static LOCK_FILE: &str = "keys.lock";
pub static ENCLAVE_KEY_PREFIX: &str = "key_";

/**
 * Directory layout:
 * - key_<address> => SealedEnclaveKeyInfo as bytes
 * - keys.lock => LOCKFILE with pid
*/
pub struct EnclaveKeyManager {
    key_dir: PathBuf,
    key_expiration_time: Duration,
}

impl EnclaveKeyManager {
    pub fn new(home_dir: &Path) -> Result<Self, Error> {
        let key_dir = home_dir.join(ENCLAVE_KEYS_DIR);

        if !key_dir.exists() {
            fs::create_dir_all(&key_dir)?;
            info!("created keys directory: {:?}", key_dir);
        }

        Ok(Self {
            key_dir,
            // TODO make expiration time configurable
            key_expiration_time: Duration::from_secs(60 * 60 * 24 * 60),
        })
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

    pub fn save_avr(
        &self,
        address: Address,
        avr: EndorsedAttestationVerificationReport,
    ) -> Result<(), Error> {
        let _lock = self.lock_blocking()?;
        let mut ski = self.load(address)?;
        // assert!(ski.avr.is_none());
        ski.avr = Some(avr);
        let bz = serde_json::to_vec(&ski)?;
        self.create_file(address, &bz)
    }

    pub fn available_keys(&self) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let _lock = self.lock_blocking()?;
        let mut skis = Vec::new();
        for entry in fs::read_dir(&self.key_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(ENCLAVE_KEY_PREFIX) {
                    let k = self.read_file(entry.path())?;
                    if k.avr.is_some() && self.is_available_key(&k)? {
                        skis.push(k);
                    }
                }
            }
        }
        Ok(skis)
    }

    pub fn all_keys(&self) -> Result<Vec<Address>, Error> {
        let _lock = self.lock_blocking()?;
        let mut skis = Vec::new();
        for entry in fs::read_dir(&self.key_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(ENCLAVE_KEY_PREFIX) {
                    skis.push(Address::from_hex_string(
                        name.strip_prefix(ENCLAVE_KEY_PREFIX).unwrap(),
                    )?);
                }
            }
        }
        Ok(skis)
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
        self.key_dir.join(format!("{ENCLAVE_KEY_PREFIX}{address}"))
    }

    fn is_available_key(&self, ski: &SealedEnclaveKeyInfo) -> Result<bool, Error> {
        if let Some(eavr) = ski.avr.as_ref() {
            let quote = eavr.get_avr()?.parse_quote()?;
            let now = Time::now();
            Ok(now < (quote.attestation_time + self.key_expiration_time)?)
        } else {
            Ok(false)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKeyInfo {
    pub address: Address,
    pub sealed_ek: SealedEnclaveKey,
    pub avr: Option<EndorsedAttestationVerificationReport>,
}

impl TryFrom<SealedEnclaveKeyInfo> for ProtoEnclaveKeyInfo {
    type Error = Error;
    fn try_from(value: SealedEnclaveKeyInfo) -> Result<Self, Self::Error> {
        let eavr = value
            .avr
            .ok_or_else(|| Error::unattested_enclave_key(format!("address={}", value.address)))?;
        let attestation_time = eavr.get_avr()?.parse_quote()?.attestation_time;
        Ok(Self {
            enclave_key_address: value.address.into(),
            attestation_time: attestation_time.as_unix_timestamp_secs(),
            report: eavr.avr,
            signature: eavr.signature,
            signing_cert: eavr.signing_cert,
            extension: Default::default(),
        })
    }
}

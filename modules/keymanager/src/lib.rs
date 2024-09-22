pub mod errors;
pub use crate::errors::Error;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::{Address, SealedEnclaveKey};
use lcp_types::proto::lcp::service::enclave::v1::EnclaveKeyInfo as ProtoEnclaveKeyInfo;
use lcp_types::{Mrenclave, Time};
use log::*;
use rusqlite::{params, types::Type, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::{path::Path, time::Duration};

pub static KEY_MANAGER_DB: &str = "km.sqlite";

pub struct EnclaveKeyManager {
    conn: Mutex<Connection>,
}

impl EnclaveKeyManager {
    pub fn new(home_dir: &Path) -> Result<Self, Error> {
        let km_db = home_dir.join(KEY_MANAGER_DB);
        let db_exists = km_db.exists();
        let conn = Mutex::new(Connection::open(&km_db)?);
        let this = Self { conn };
        if !db_exists {
            this.init_db()?;
            info!("initialized Key Manager: {:?}", km_db);
        }
        Ok(this)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, Error> {
        let conn = Mutex::new(Connection::open_in_memory()?);
        let this = Self { conn };
        this.init_db()?;
        Ok(this)
    }

    fn init_db(&self) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE enclave_keys (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                ek_address TEXT NOT NULL UNIQUE,
                ek_sealed BLOB NOT NULL,
                mrenclave TEXT NOT NULL,
                avr TEXT,
                signature BLOB,
                signing_cert BLOB,
                attested_at TEXT,
                created_at TEXT NOT NULL DEFAULT (DATETIME('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (DATETIME('now', 'localtime'))
            );
            CREATE UNIQUE INDEX index_ek_address on enclave_keys(ek_address);
            COMMIT;
            "#,
        )?;
        Ok(())
    }

    /// Load a sealed enclave key by address
    pub fn load(&self, address: Address) -> Result<SealedEnclaveKeyInfo, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT ek_sealed, mrenclave, avr, signature, signing_cert FROM enclave_keys WHERE ek_address = ?1",
        )?;
        let key_info = stmt.query_row(params![address.to_hex_string()], |row| {
            Ok(SealedEnclaveKeyInfo {
                address,
                sealed_ek: SealedEnclaveKey::new_from_bytes(row.get::<_, Vec<u8>>(0)?.as_slice())
                    .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, e.into())
                })?,
                mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(1)?).unwrap(),
                avr: match (row.get(2), row.get(3), row.get(4)) {
                    (Ok(None), Ok(None), Ok(None)) => None,
                    (Ok(Some(avr)), Ok(Some(signature)), Ok(Some(signing_cert))) => {
                        Some(EndorsedAttestationVerificationReport {
                            avr,
                            signature,
                            signing_cert,
                        })
                    }
                    (e0, e1, e2) => [e0.err(), e1.err(), e2.err()]
                        .into_iter()
                        .find_map(|e| e.map(Err))
                        .unwrap()?,
                },
            })
        })?;
        Ok(key_info)
    }

    /// Save a sealed enclave key
    pub fn save(
        &self,
        address: Address,
        sealed_ek: SealedEnclaveKey,
        mrenclave: Mrenclave,
    ) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            "INSERT INTO enclave_keys (ek_address, ek_sealed, mrenclave) VALUES (?1, ?2, ?3)",
        )?;
        let _ = stmt.execute(params![
            address.to_hex_string(),
            sealed_ek.to_vec(),
            mrenclave.to_hex_string()
        ])?;
        Ok(())
    }

    /// Update the attestation verification report for the enclave key
    pub fn save_avr(
        &self,
        address: Address,
        avr: EndorsedAttestationVerificationReport,
    ) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let attested_at = avr.get_avr()?.attestation_time()?;
        // update avr and attested_at and signature and sigining_cert
        let mut stmt = conn.prepare(
            "UPDATE enclave_keys SET avr = ?1, attested_at = ?2, signature = ?3, signing_cert = ?4 WHERE ek_address = ?5",
        )?;
        stmt.execute(params![
            avr.avr,
            attested_at.as_unix_timestamp_secs(),
            avr.signature,
            avr.signing_cert,
            address.to_hex_string()
        ])?;
        Ok(())
    }

    /// Returns a list of available enclave keys
    pub fn available_keys(&self, mrenclave: Mrenclave) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT ek_address, ek_sealed, mrenclave, avr, signature, signing_cert
            FROM enclave_keys
            WHERE attested_at IS NOT NULL AND mrenclave = ?1
            ORDER BY attested_at DESC
            "#,
        )?;
        let key_infos = stmt
            .query_map(params![mrenclave.to_hex_string()], |row| {
                Ok(SealedEnclaveKeyInfo {
                    address: Address::from_hex_string(&row.get::<_, String>(0)?).unwrap(),
                    sealed_ek: SealedEnclaveKey::new_from_bytes(
                        row.get::<_, Vec<u8>>(1)?.as_slice(),
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, e.into())
                    })?,
                    mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(2)?).unwrap(),
                    avr: Some(EndorsedAttestationVerificationReport {
                        avr: row.get(3)?,
                        signature: row.get(4)?,
                        signing_cert: row.get(5)?,
                    }),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(key_infos)
    }

    /// Returns a list of all enclave keys
    pub fn all_keys(&self) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT ek_address, ek_sealed, mrenclave, avr, signature, signing_cert FROM enclave_keys ORDER BY updated_at DESC",
        )?;
        let key_infos = stmt
            .query_map(params![], |row| {
                Ok(SealedEnclaveKeyInfo {
                    address: Address::from_hex_string(&row.get::<_, String>(0)?).unwrap(),
                    sealed_ek: SealedEnclaveKey::new_from_bytes(
                        row.get::<_, Vec<u8>>(1)?.as_slice(),
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, e.into())
                    })?,
                    mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(2)?).unwrap(),
                    avr: match (row.get(3), row.get(4), row.get(5)) {
                        (Ok(None), Ok(None), Ok(None)) => None,
                        (Ok(Some(avr)), Ok(Some(signature)), Ok(Some(signing_cert))) => {
                            Some(EndorsedAttestationVerificationReport {
                                avr,
                                signature,
                                signing_cert,
                            })
                        }
                        (e0, e1, e2) => [e0.err(), e1.err(), e2.err()]
                            .into_iter()
                            .find_map(|e| e.map(Err))
                            .unwrap()?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(key_infos)
    }

    /// Prune keys after the expiration time(secs) from the attestation time.
    pub fn prune(&self, expiration_time: u64) -> Result<usize, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let expired = (Time::now() - Duration::from_secs(expiration_time))?;
        let mut stmt = conn.prepare("DELETE FROM enclave_keys WHERE attested_at <= ?1")?;
        let count = stmt.execute(params![expired.as_unix_timestamp_secs()])?;
        Ok(count)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKeyInfo {
    pub address: Address,
    pub sealed_ek: SealedEnclaveKey,
    pub mrenclave: Mrenclave,
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

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_report::AttestationVerificationReport;
    use chrono::{DateTime, Duration, Utc};
    use rand::RngCore;

    #[test]
    fn test_keys() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let address_0 = {
            let address = create_address();
            let sealed_ek = create_sealed_sk();
            km.save(address, sealed_ek, mrenclave).unwrap();
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 0);
            let avr = create_eavr(get_time(Duration::zero()));
            km.save_avr(address, avr).unwrap();
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            address
        };
        {
            let address = create_address();
            let sealed_ek = create_sealed_sk();
            km.save(address, sealed_ek, mrenclave).unwrap();
            assert_eq!(km.all_keys().unwrap().len(), 2);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            let avr = create_eavr(get_time(Duration::minutes(1)));
            km.save_avr(address, avr).unwrap();
            assert_eq!(km.all_keys().unwrap().len(), 2);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 2);
        }
        // there are no keys available for the mrenclave
        assert_eq!(km.available_keys(create_mrenclave()).unwrap().len(), 0);
        assert_eq!(km.prune(30).unwrap(), 1);
        assert_eq!(km.all_keys().unwrap().len(), 1);
        assert_eq!(
            km.available_keys(mrenclave)
                .unwrap()
                .first()
                .unwrap()
                .address,
            address_0
        );
    }

    fn get_time(d: Duration) -> DateTime<Utc> {
        Utc::now().checked_sub_signed(d).unwrap()
    }

    fn create_mrenclave() -> Mrenclave {
        Mrenclave(rand::random())
    }

    fn create_sealed_sk() -> SealedEnclaveKey {
        let mut sealed_sk = [0u8; 592];
        rand::thread_rng().fill_bytes(&mut sealed_sk);
        SealedEnclaveKey::new_from_bytes(&sealed_sk).unwrap()
    }

    fn create_address() -> Address {
        let bz: [u8; 20] = rand::random();
        let addr = Address::try_from(bz.as_slice()).unwrap();
        addr
    }

    fn create_eavr(timestamp: DateTime<Utc>) -> EndorsedAttestationVerificationReport {
        EndorsedAttestationVerificationReport {
            avr: AttestationVerificationReport {
                version: 4,
                timestamp: format!(
                    "{}000",
                    timestamp
                        .format("%Y-%m-%dT%H:%M:%S%.f%z")
                        .to_string()
                        .strip_suffix("+0000")
                        .unwrap()
                ),
                ..Default::default()
            }
            .to_canonical_json()
            .unwrap(),
            ..Default::default()
        }
    }
}

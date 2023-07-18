pub mod errors;
pub use crate::errors::Error;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::{Address, SealedEnclaveKey};
use lcp_proto::lcp::service::enclave::v1::EnclaveKeyInfo as ProtoEnclaveKeyInfo;
use lcp_types::{Mrenclave, Time};
use log::*;
use rusqlite::{params, types::Type, Connection};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, path::Path, time::Duration};

pub static KEY_MANAGER_DB: &str = "km.sqlite";

pub struct EnclaveKeyManager {
    conn: Connection,
}

impl EnclaveKeyManager {
    pub fn new(home_dir: &Path) -> Result<Self, Error> {
        let km_db = home_dir.join(KEY_MANAGER_DB);
        let db_exists = km_db.exists();
        let conn = Connection::open(&km_db)?;
        let this = Self { conn };
        if !db_exists {
            this.setup()?;
            info!("initialized DB: {:?}", km_db);
        }
        Ok(this)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()?;
        let this = Self { conn };
        this.setup()?;
        Ok(this)
    }

    fn setup(&self) -> Result<(), Error> {
        self.conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE enclave_keys (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                ek_address VARCHAR NOT NULL UNIQUE,
                ek_sealed TEXT NOT NULL,
                mrenclave VARCHAR NOT NULL,
                avr TEXT,
                signature TEXT,
                signing_cert TEXT,
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
        let mut stmt = self.conn.prepare(
            "SELECT ek_sealed, mrenclave, avr, signature, signing_cert FROM enclave_keys WHERE ek_address = ?1",
        )?;
        let key_info = stmt.query_row(params![address.to_hex_string()], |row| {
            Ok(SealedEnclaveKeyInfo {
                address,
                sealed_ek: SealedEnclaveKey::new_from_bytes(row.get::<_, Vec<u8>>(0)?.as_slice())
                    .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, e.into())
                })?,
                mrenclave: Mrenclave(row.get(1)?),
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
        let mut stmt = self.conn.prepare(
            "INSERT INTO enclave_keys (ek_address, ek_sealed, mrenclave) VALUES (?1, ?2, ?3)",
        )?;
        let _ = stmt.execute(params![
            address.to_hex_string(),
            sealed_ek.to_vec(),
            mrenclave.deref()
        ])?;
        Ok(())
    }

    /// Update the attestation verification report for the enclave key
    pub fn save_avr(
        &self,
        address: Address,
        avr: EndorsedAttestationVerificationReport,
    ) -> Result<(), Error> {
        let attested_at = avr.get_avr()?.attestation_time()?;
        // update avr and attested_at and signature and sigining_cert
        let mut stmt = self.conn.prepare(
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
        let mut stmt = self.conn.prepare(
            r#"
            SELECT ek_address, ek_sealed, mrenclave, avr, signature, signing_cert
            FROM enclave_keys
            WHERE attested_at IS NOT NULL AND mrenclave = ?1
            ORDER BY attested_at DESC
            "#,
        )?;
        let key_infos = stmt
            .query_map(params![mrenclave.deref()], |row| {
                Ok(SealedEnclaveKeyInfo {
                    address: Address::from_hex_string(&row.get::<_, String>(0)?).unwrap(),
                    sealed_ek: SealedEnclaveKey::new_from_bytes(
                        row.get::<_, Vec<u8>>(1)?.as_slice(),
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, e.into())
                    })?,
                    mrenclave: Mrenclave(row.get(2)?),
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
        let mut stmt = self.conn.prepare(
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
                    mrenclave: Mrenclave(row.get(2)?),
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

    /// Prune keys after the expiration time from the attestation time.
    pub fn prune(&self, expiration: u64) -> Result<usize, Error> {
        let expiration = (Time::now() - Duration::from_secs(expiration))?;
        let mut stmt = self
            .conn
            .prepare("DELETE FROM enclave_keys WHERE attested_at < ?1")?;
        let count = stmt.execute(params![expiration.as_unix_timestamp_secs()])?;
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

    #[test]
    fn test_km() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let sealed_ek = SealedEnclaveKey::new_from_bytes(&[0u8; 592]).unwrap();
        let address = Address::from_hex_string("aabbccddeeff0011223344556677889900112233").unwrap();
        let mrenclave = Mrenclave([0u8; 32]);
        km.save(address, sealed_ek, mrenclave).unwrap();

        assert_eq!(km.available_keys(mrenclave).unwrap().len(), 0);

        // generate avr and call save_avr
        let avr = EndorsedAttestationVerificationReport {
            avr: AttestationVerificationReport {
                version: 4,
                timestamp: "2023-07-13T02:37:33.881000".to_owned(),
                ..Default::default()
            }
            .to_canonical_json()
            .unwrap(),
            ..Default::default()
        };
        km.save_avr(address, avr).unwrap();

        assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
        assert_eq!(km.prune(0).unwrap(), 1);
        assert_eq!(km.available_keys(mrenclave).unwrap().len(), 0);

        println!("{}", Time::now().to_rfc3339());
    }
}

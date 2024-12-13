pub mod errors;
pub use crate::errors::Error;
use anyhow::anyhow;
use attestation_report::{DCAPQuote, IASSignedReport, ReportData, VerifiableQuote};
use crypto::{Address, SealedEnclaveKey};
use lcp_types::{
    deserialize_bytes, proto::lcp::service::enclave::v1::EnclaveKeyInfo as ProtoEnclaveKeyInfo,
    serialize_bytes, BytesTransmuter, Mrenclave, Time,
};
use log::*;
use rusqlite::{params, types::Type, Connection};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::sgx_report_t;
use std::{path::Path, sync::Mutex, time::Duration};

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
                report BLOB NOT NULL,
                ias_report TEXT,
                dcap_quote TEXT,
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
            r#"
            SELECT ek_sealed, mrenclave, report, ias_report, dcap_quote
            FROM enclave_keys
            WHERE ek_address = ?1
            "#,
        )?;
        let key_info = stmt.query_row(params![address.to_hex_string()], |row| {
            Ok(SealedEnclaveKeyInfo {
                address,
                sealed_ek: SealedEnclaveKey::new_from_bytes(row.get::<_, Vec<u8>>(0)?.as_slice())
                    .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        Type::Blob,
                        anyhow!("sealed_ek: {:?}", e).into(),
                    )
                })?,
                mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(1)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        Type::Text,
                        anyhow!("mrenclave: {:?}", e).into(),
                    )
                })?,
                report: deserialize_bytes(&row.get::<_, Vec<u8>>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        Type::Blob,
                        anyhow!("report: {:?}", e).into(),
                    )
                })?,
                ias_report: match row.get::<_, Option<String>>(3) {
                    Ok(None) => None,
                    Ok(Some(avr)) => Some(IASSignedReport::from_json(&avr).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            Type::Text,
                            anyhow!("ias_report: {:?}", e).into(),
                        )
                    })?),
                    Err(e) => return Err(e),
                },
                dcap_quote: match row.get::<_, Option<String>>(4) {
                    Ok(None) => None,
                    Ok(Some(dq)) => Some(DCAPQuote::from_json(&dq).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Text,
                            anyhow!("dcap_quote: {:?}", e).into(),
                        )
                    })?),
                    Err(e) => return Err(e),
                },
            })
        })?;
        Ok(key_info)
    }

    /// Save a sealed enclave key
    pub fn save(&self, sealed_ek: SealedEnclaveKey, report: sgx_report_t) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            INSERT INTO enclave_keys(ek_address, ek_sealed, mrenclave, report)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )?;
        let rd = ReportData::from(report.body.report_data);
        let _ = stmt.execute(params![
            rd.enclave_key().to_hex_string(),
            sealed_ek.to_vec(),
            Mrenclave::from(report.body.mr_enclave).to_hex_string(),
            serialize_bytes(&report),
        ])?;
        Ok(())
    }

    /// Update the attestation verification report for the enclave key
    pub fn save_verifiable_quote(
        &self,
        address: Address,
        vquote: VerifiableQuote,
    ) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;

        match vquote {
            VerifiableQuote::IAS(ias_report) => {
                let mut stmt = conn.prepare(
                    r#"
                    UPDATE enclave_keys
                    SET ias_report = ?1, attested_at = ?2
                    WHERE ek_address = ?3
                    "#,
                )?;
                stmt.execute(params![
                    ias_report.to_json()?,
                    ias_report
                        .get_avr()?
                        .attestation_time()?
                        .as_unix_timestamp_secs(),
                    address.to_hex_string()
                ])?;
            }
            VerifiableQuote::DCAP(dcap_quote) => {
                let mut stmt = conn.prepare(
                    r#"
                    UPDATE enclave_keys
                    SET dcap_quote = ?1, attested_at = ?2
                    WHERE ek_address = ?3
                    "#,
                )?;
                stmt.execute(params![
                    dcap_quote.to_json()?,
                    dcap_quote.attested_at.as_unix_timestamp_secs(),
                    address.to_hex_string()
                ])?;
            }
        }
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
            SELECT ek_address, ek_sealed, mrenclave, report, ias_report, dcap_quote
            FROM enclave_keys
            WHERE attested_at IS NOT NULL AND mrenclave = ?1
            ORDER BY attested_at DESC
            "#,
        )?;
        let key_infos = stmt
            .query_map(params![mrenclave.to_hex_string()], |row| {
                Ok(SealedEnclaveKeyInfo {
                    address: Address::from_hex_string(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            Type::Text,
                            anyhow!("address: {:?}", e).into(),
                        )
                    })?,
                    sealed_ek: SealedEnclaveKey::new_from_bytes(
                        row.get::<_, Vec<u8>>(1)?.as_slice(),
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            Type::Blob,
                            anyhow!("sealed_ek: {:?}", e).into(),
                        )
                    })?,
                    mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(2)?).map_err(
                        |e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                Type::Text,
                                anyhow!("mrenclave: {:?}", e).into(),
                            )
                        },
                    )?,
                    report: deserialize_bytes(&row.get::<_, Vec<u8>>(3)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            Type::Blob,
                            anyhow!("report: {:?}", e).into(),
                        )
                    })?,
                    ias_report: Some(
                        IASSignedReport::from_json(&row.get::<_, String>(4)?).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                Type::Text,
                                anyhow!("ias_report: {:?}", e).into(),
                            )
                        })?,
                    ),
                    dcap_quote: match row.get::<_, Option<String>>(5) {
                        Ok(None) => None,
                        Ok(Some(dq)) => Some(DCAPQuote::from_json(&dq).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                5,
                                Type::Text,
                                anyhow!("dcap_quote: {:?}", e).into(),
                            )
                        })?),
                        Err(e) => return Err(e),
                    },
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
            r#"
            SELECT ek_address, ek_sealed, mrenclave, report, ias_report, dcap_quote
            FROM enclave_keys
            ORDER BY updated_at DESC
            "#,
        )?;
        let key_infos = stmt
            .query_map(params![], |row| {
                Ok(SealedEnclaveKeyInfo {
                    address: Address::from_hex_string(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            Type::Text,
                            anyhow!("address: {:?}", e).into(),
                        )
                    })?,
                    sealed_ek: SealedEnclaveKey::new_from_bytes(
                        row.get::<_, Vec<u8>>(1)?.as_slice(),
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            Type::Blob,
                            anyhow!("sealed_ek: {:?}", e).into(),
                        )
                    })?,
                    mrenclave: Mrenclave::from_hex_string(&row.get::<_, String>(2)?).map_err(
                        |e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                Type::Text,
                                anyhow!("mrenclave: {:?}", e).into(),
                            )
                        },
                    )?,
                    report: deserialize_bytes(&row.get::<_, Vec<u8>>(3)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            Type::Blob,
                            anyhow!("report: {:?}", e).into(),
                        )
                    })?,
                    ias_report: match row.get::<_, Option<String>>(4) {
                        Ok(None) => None,
                        Ok(Some(avr)) => Some(IASSignedReport::from_json(&avr).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                Type::Text,
                                anyhow!("ias_report: {:?}", e).into(),
                            )
                        })?),
                        Err(e) => return Err(e),
                    },
                    dcap_quote: match row.get::<_, Option<String>>(5) {
                        Ok(None) => None,
                        Ok(Some(dq)) => Some(DCAPQuote::from_json(&dq).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                5,
                                Type::Text,
                                anyhow!("dcap_quote: {:?}", e).into(),
                            )
                        })?),
                        Err(e) => return Err(e),
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

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKeyInfo {
    pub sealed_ek: SealedEnclaveKey,
    pub address: Address,
    pub mrenclave: Mrenclave,
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
    pub ias_report: Option<IASSignedReport>,
    pub dcap_quote: Option<DCAPQuote>,
}

impl TryFrom<SealedEnclaveKeyInfo> for ProtoEnclaveKeyInfo {
    type Error = Error;
    fn try_from(value: SealedEnclaveKeyInfo) -> Result<Self, Self::Error> {
        let ias_report = value
            .ias_report
            .ok_or_else(|| Error::unattested_enclave_key(format!("address={}", value.address)))?;
        let attestation_time = ias_report.get_avr()?.parse_quote()?.attestation_time;
        Ok(Self {
            enclave_key_address: value.address.into(),
            attestation_time: attestation_time.as_unix_timestamp_secs(),
            report: ias_report.avr,
            signature: ias_report.signature,
            signing_cert: ias_report.signing_cert,
            extension: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_report::IASAttestationVerificationReport;
    use chrono::{DateTime, Duration, Utc};
    use rand::RngCore;

    #[test]
    fn test_keys() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let address_0 = {
            let address = create_address();
            let report = create_report(mrenclave, address);
            let sealed_ek = create_sealed_sk();
            assert_eq!(km.all_keys().unwrap().len(), 0);
            km.save(sealed_ek, report).unwrap();
            assert!(km.load(address).unwrap().ias_report.is_none());
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 0);
            let ias_report = create_ias_report(get_time(Duration::zero()));
            km.save_verifiable_quote(address, ias_report.into())
                .unwrap();
            assert!(km.load(address).unwrap().ias_report.is_some());
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            address
        };
        {
            let address = create_address();
            let report = create_report(mrenclave, address);
            let sealed_ek = create_sealed_sk();
            assert_eq!(km.all_keys().unwrap().len(), 1);
            km.save(sealed_ek, report).unwrap();
            assert!(km.load(address).unwrap().ias_report.is_none());
            assert_eq!(km.all_keys().unwrap().len(), 2);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            let ias_report = create_ias_report(get_time(Duration::minutes(1)));
            km.save_verifiable_quote(address, ias_report.into())
                .unwrap();
            assert!(km.load(address).unwrap().ias_report.is_some());
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

    fn create_report(mrenclave: Mrenclave, ek_addr: Address) -> sgx_report_t {
        let mut report = sgx_report_t::default();
        report.body.mr_enclave = mrenclave.into();
        report.body.report_data = ReportData::new(ek_addr, None).into();
        report
    }

    fn create_address() -> Address {
        let bz: [u8; 20] = rand::random();
        let addr = Address::try_from(bz.as_slice()).unwrap();
        addr
    }

    fn create_ias_report(timestamp: DateTime<Utc>) -> IASSignedReport {
        IASSignedReport {
            avr: IASAttestationVerificationReport {
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

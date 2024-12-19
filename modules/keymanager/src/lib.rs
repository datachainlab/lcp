pub mod errors;
pub use crate::errors::Error;
use anyhow::anyhow;
use attestation_report::{RAQuote, RAType, ReportData};
use crypto::{Address, SealedEnclaveKey};
use lcp_types::{
    deserialize_bytes,
    proto::lcp::service::enclave::v1::{
        enclave_key_info, DcapEnclaveKeyInfo, EnclaveKeyInfo as ProtoEnclaveKeyInfo,
        IasEnclaveKeyInfo,
    },
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
                ra_type INTEGER NOT NULL,
                ra_quote TEXT,
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
            SELECT ek_sealed, mrenclave, report, ra_type, ra_quote
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
                ra_type: RAType::from_u32(row.get::<_, i64>(3)? as u32).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        Type::Integer,
                        anyhow!("ra_type: {:?}", e).into(),
                    )
                })?,
                ra_quote: match row.get::<_, Option<String>>(4) {
                    Ok(None) => None,
                    Ok(Some(ra_quote)) => Some(RAQuote::from_json(&ra_quote).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Text,
                            anyhow!("ra_quote: {:?}", e).into(),
                        )
                    })?),
                    Err(e) => return Err(e),
                },
            })
        })?;
        Ok(key_info)
    }

    /// Save a sealed enclave key
    pub fn save(
        &self,
        sealed_ek: SealedEnclaveKey,
        report: sgx_report_t,
        ra_type: RAType,
    ) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            INSERT INTO enclave_keys(ek_address, ek_sealed, mrenclave, report, ra_type)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )?;
        let rd = ReportData::from(report.body.report_data);
        let _ = stmt.execute(params![
            rd.enclave_key().to_hex_string(),
            sealed_ek.to_vec(),
            Mrenclave::from(report.body.mr_enclave).to_hex_string(),
            serialize_bytes(&report),
            ra_type.as_u32()
        ])?;
        Ok(())
    }

    /// Update the attestation verification report for the enclave key
    pub fn save_ra_quote(&self, address: Address, vquote: RAQuote) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            UPDATE enclave_keys
            SET ra_quote = ?1, attested_at = ?2
            WHERE ek_address = ?3
            "#,
        )?;
        stmt.execute(params![
            vquote.to_json()?,
            vquote.attested_at()?.as_unix_timestamp_secs(),
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
            SELECT ek_address, ek_sealed, mrenclave, report, ra_type, ra_quote
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
                    ra_type: RAType::from_u32(row.get::<_, i64>(4)? as u32).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Integer,
                            anyhow!("ra_type: {:?}", e).into(),
                        )
                    })?,
                    ra_quote: match row.get::<_, Option<String>>(5) {
                        Ok(None) => None,
                        Ok(Some(ra_quote)) => Some(RAQuote::from_json(&ra_quote).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                5,
                                Type::Text,
                                anyhow!("ra_quote: {:?}", e).into(),
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
            SELECT ek_address, ek_sealed, mrenclave, report, ra_type, ra_quote
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
                    ra_type: RAType::from_u32(row.get::<_, i64>(4)? as u32).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Integer,
                            anyhow!("ra_type: {:?}", e).into(),
                        )
                    })?,
                    ra_quote: match row.get::<_, Option<String>>(5) {
                        Ok(None) => None,
                        Ok(Some(avr)) => Some(RAQuote::from_json(&avr).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                5,
                                Type::Text,
                                anyhow!("ra_quote: {:?}", e).into(),
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
    pub ra_type: RAType,
    pub ra_quote: Option<RAQuote>,
}

impl TryFrom<SealedEnclaveKeyInfo> for ProtoEnclaveKeyInfo {
    type Error = Error;
    fn try_from(value: SealedEnclaveKeyInfo) -> Result<Self, Self::Error> {
        match value.ra_quote {
            Some(RAQuote::IAS(report)) => {
                let attestation_time = report
                    .get_avr()?
                    .attestation_time()?
                    .as_unix_timestamp_secs();
                Ok(ProtoEnclaveKeyInfo {
                    key_info: Some(enclave_key_info::KeyInfo::Ias(IasEnclaveKeyInfo {
                        enclave_key_address: value.address.into(),
                        attestation_time,
                        report: report.avr,
                        signature: report.signature,
                        signing_cert: report.signing_cert,
                    })),
                })
            }
            Some(RAQuote::DCAP(dcap)) => {
                let attestation_time = dcap.attested_at.as_unix_timestamp_secs();
                Ok(ProtoEnclaveKeyInfo {
                    key_info: Some(enclave_key_info::KeyInfo::Dcap(DcapEnclaveKeyInfo {
                        enclave_key_address: value.address.into(),
                        quote: dcap.raw,
                        fmspc: dcap.fmspc.to_vec(),
                        attestation_time,
                        tcb_status: dcap.tcb_status,
                        advisory_ids: dcap.advisory_ids,
                        collateral: Some(dcap.collateral),
                    })),
                })
            }
            None => Err(Error::unattested_enclave_key(format!(
                "address={}",
                value.address
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_report::{IASAttestationVerificationReport, IASSignedReport};
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
            km.save(sealed_ek, report, RAType::IAS).unwrap();
            assert!(km.load(address).unwrap().ra_quote.is_none());
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 0);
            let ias_report = create_ias_report(get_time(Duration::zero()));
            km.save_ra_quote(address, ias_report.into()).unwrap();
            assert!(km.load(address).unwrap().ra_quote.is_some());
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            address
        };
        {
            let address = create_address();
            let report = create_report(mrenclave, address);
            let sealed_ek = create_sealed_sk();
            assert_eq!(km.all_keys().unwrap().len(), 1);
            km.save(sealed_ek, report, RAType::IAS).unwrap();
            assert!(km.load(address).unwrap().ra_quote.is_none());
            assert_eq!(km.all_keys().unwrap().len(), 2);
            assert_eq!(km.available_keys(mrenclave).unwrap().len(), 1);
            let ias_report = create_ias_report(get_time(Duration::minutes(1)));
            km.save_ra_quote(address, ias_report.into()).unwrap();
            assert!(km.load(address).unwrap().ra_quote.is_some());
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

    #[test]
    fn test_key_info_conversion() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address);
        km.save(sealed_ek, report, RAType::IAS).unwrap();
        let key_info = km.load(address).unwrap();
        assert!(ProtoEnclaveKeyInfo::try_from(key_info).is_err());
        let ias_report = create_ias_report(get_time(Duration::minutes(1)));
        km.save_ra_quote(address, ias_report.into()).unwrap();
        let key_info = km.load(address).unwrap();
        let res = ProtoEnclaveKeyInfo::try_from(key_info);
        assert!(res.is_ok(), "{:?}", res);
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

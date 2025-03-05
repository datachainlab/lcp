pub mod errors;
pub use crate::errors::Error;
use anyhow::anyhow;
use attestation_report::{is_enclave_debug_enabled, QEType, RAQuote, RAType, ReportData};
use crypto::{Address, SealedEnclaveKey};
use lcp_types::{
    deserialize_bytes,
    proto::lcp::service::enclave::v1::{
        enclave_key_info, zkvm_proof, DcapEnclaveKeyInfo, EnclaveKeyInfo as ProtoEnclaveKeyInfo,
        IasEnclaveKeyInfo, Risc0ZkvmProof, ZkdcapEncalveKeyInfo, ZkvmProof,
    },
    serialize_bytes, BytesTransmuter, Mrenclave, Time,
};
use log::*;
use rusqlite::{params, types::Type, Connection};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::sgx_report_t;
use std::{path::Path, sync::Mutex};

/// Key Manager database file name
pub const KEY_MANAGER_DB: &str = "km.sqlite";

/// SQL statement to create the enclave_keys table
pub const CREATE_ENCLAVE_KEYS_TABLE: &str = r#"
CREATE TABLE enclave_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE,
    sealed_key BLOB NOT NULL,
    mrenclave TEXT NOT NULL,
    report BLOB NOT NULL,
    enclave_debug INTEGER NOT NULL,
    qe_type INTEGER NOT NULL,
    ra_type INTEGER,
    ra_quote TEXT,
    valid_from INTEGER,
    valid_to INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
CREATE UNIQUE INDEX idx_address ON enclave_keys(address);
"#;

/// Enclave Key Manager to manage sealed enclave key and attestation verification reports for the keys
pub struct EnclaveKeyManager {
    conn: Mutex<Connection>,
}

impl EnclaveKeyManager {
    /// Create a new Key Manager instance
    ///
    /// # Arguments
    /// - `home_dir` - The directory where the LCP's home directory is located
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

    /// Create a new Key Manager instance with an in-memory database
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
        conn.execute_batch(format!("BEGIN;{}COMMIT;", CREATE_ENCLAVE_KEYS_TABLE).as_str())?;
        Ok(())
    }

    /// Load a sealed enclave key by address
    ///
    /// # Arguments
    /// * `address` - The address of the enclave key
    pub fn load(&self, address: Address) -> Result<SealedEnclaveKeyInfo, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT sealed_key, mrenclave, report, qe_type, enclave_debug, ra_quote
            FROM enclave_keys
            WHERE address = ?1
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
                qe_type: QEType::from_u32(row.get::<_, i64>(3)? as u32).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        Type::Integer,
                        anyhow!("qe_type: {:?}", e).into(),
                    )
                })?,
                enclave_debug: row.get::<_, i64>(4)? != 0,
                ra_quote: match row.get::<_, Option<String>>(5) {
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
    ///
    /// # Arguments
    /// * `sealed_key` - The sealed key
    /// * `report` - The attestation verification report
    /// * `qe_type` - The quote enclave type
    pub fn save(
        &self,
        sealed_key: SealedEnclaveKey,
        report: sgx_report_t,
        qe_type: QEType,
    ) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            INSERT INTO enclave_keys(address, sealed_key, mrenclave, report, enclave_debug, qe_type)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;
        stmt.execute(params![
            ReportData::from(report.body.report_data)
                .enclave_key()
                .to_hex_string(),
            sealed_key.to_vec(),
            Mrenclave::from(report.body.mr_enclave).to_hex_string(),
            serialize_bytes(&report),
            is_enclave_debug_enabled(&report.body),
            qe_type.as_u32()
        ])?;
        Ok(())
    }

    /// Update the attestation verification report for the enclave key
    ///
    /// # Arguments
    /// * `address` - The address of the enclave key
    /// * `ra_quote` - The remote attestation quote
    pub fn update_ra_quote(&self, address: Address, ra_quote: RAQuote) -> Result<(), Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            UPDATE enclave_keys
            SET ra_type = ?1, ra_quote = ?2, valid_from = ?3, valid_to = ?4
            WHERE address = ?5 AND ra_type IS NULL
            "#,
        )?;
        let size = stmt.execute(params![
            ra_quote.ra_type().as_u32(),
            ra_quote.to_json()?,
            ra_quote.valid_from()?.as_unix_timestamp_secs(),
            ra_quote.valid_to()?.as_unix_timestamp_secs(),
            address.to_hex_string()
        ])?;
        if size == 0 {
            Err(Error::unattested_enclave_key_not_found(address))
        } else {
            Ok(())
        }
    }

    /// Returns a list of attested enclave keys
    ///
    /// The order of the returned keys is by the `valid_to` timestamp in descending order.
    ///
    /// # Arguments
    /// * `mrenclave` - The MRENCLAVE value of the enclave
    /// * `enclave_debug` - Whether the enclave is enabled for debug
    /// * `ra_type` - The type of remote attestation. If None, all available keys are returned.
    ///
    /// # Returns
    /// Returns a list of attested enclave keys
    pub fn available_keys(
        &self,
        mrenclave: Mrenclave,
        enclave_debug: bool,
        ra_type: Option<RAType>,
    ) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;

        let (mut stmt, params) = if let Some(ra_type) = ra_type {
            (
                conn.prepare(
                    r#"
                SELECT address, sealed_key, mrenclave, report, qe_type, enclave_debug, ra_quote
                FROM enclave_keys
                WHERE valid_to IS NOT NULL AND mrenclave = ?1 AND enclave_debug = ?2 AND ra_type = ?3
                ORDER BY valid_to DESC
                "#,
                )?,
                params![mrenclave.to_hex_string(), enclave_debug, ra_type.as_u32()],
            )
        } else {
            (
                conn.prepare(
                    r#"
                SELECT address, sealed_key, mrenclave, report, qe_type, enclave_debug, ra_quote
                FROM enclave_keys
                WHERE valid_to IS NOT NULL AND mrenclave = ?1 AND enclave_debug = ?2
                ORDER BY valid_to DESC
                "#,
                )?,
                params![mrenclave.to_hex_string(), enclave_debug],
            )
        };

        let key_infos = stmt
            .query_map(params, |row| {
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
                            anyhow!("sealed_key: {:?}", e).into(),
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
                    qe_type: QEType::from_u32(row.get::<_, i64>(4)? as u32).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Integer,
                            anyhow!("qe_type: {:?}", e).into(),
                        )
                    })?,
                    enclave_debug: row.get::<_, i64>(5)? != 0,
                    ra_quote: match row.get::<_, Option<String>>(6) {
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
    ///
    /// The order of the returned keys is by the `created_at` timestamp in descending order.
    pub fn all_keys(&self) -> Result<Vec<SealedEnclaveKeyInfo>, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT address, sealed_key, mrenclave, report, qe_type, enclave_debug, ra_quote
            FROM enclave_keys
            ORDER BY created_at DESC
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
                            anyhow!("sealed_key: {:?}", e).into(),
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
                    qe_type: QEType::from_u32(row.get::<_, i64>(4)? as u32).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            Type::Integer,
                            anyhow!("qe_type: {:?}", e).into(),
                        )
                    })?,
                    enclave_debug: row.get::<_, i64>(5)? != 0,
                    ra_quote: match row.get::<_, Option<String>>(6) {
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

    /// Prunes expired keys from the database.
    ///
    /// # Arguments
    /// * `current_time` - The current time. If None, the current time is used.
    /// * `policy` - The prune policy
    ///
    /// # Returns
    /// Returns the number of keys pruned.
    pub fn prune(&self, current_time: Option<Time>, policy: PrunePolicy) -> Result<usize, Error> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::mutex_lock(e.to_string()))?;

        let current_time = current_time.unwrap_or(Time::now());
        let (mut stmt, params) = match policy {
            PrunePolicy::ExpiredCreatedAt(expiration_period) => (
                conn.prepare(
                    r#"
                    DELETE FROM enclave_keys
                    WHERE valid_to IS NULL AND created_at <= ?1
                    "#,
                )?,
                params![current_time.as_unix_timestamp_secs() - expiration_period],
            ),
            PrunePolicy::ValidTo => (
                conn.prepare(
                    r#"
                    DELETE FROM enclave_keys
                    WHERE valid_to IS NOT NULL AND valid_to <= ?1
                    "#,
                )?,
                params![current_time.as_unix_timestamp_secs()],
            ),
        };
        Ok(stmt.execute(params)?)
    }
}

/// Prune policy for the Key Manager
pub enum PrunePolicy {
    /// Prune keys based on the creation time.
    ExpiredCreatedAt(u64),
    /// Prune keys based on the `valid_to` timestamp.
    ValidTo,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKeyInfo {
    pub sealed_ek: SealedEnclaveKey,
    pub address: Address,
    pub mrenclave: Mrenclave,
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
    pub qe_type: QEType,
    pub enclave_debug: bool,
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
            Some(RAQuote::DCAP(dcap)) => Ok(ProtoEnclaveKeyInfo {
                key_info: Some(enclave_key_info::KeyInfo::Dcap(DcapEnclaveKeyInfo {
                    enclave_key_address: value.address.into(),
                    quote: dcap.raw,
                    fmspc: dcap.fmspc.to_vec(),
                    validity: dcap.validity.into(),
                    tcb_status: dcap.status,
                    advisory_ids: dcap.advisory_ids,
                    collateral: Some(dcap.collateral),
                })),
            }),
            Some(RAQuote::ZKDCAP(zkquote)) => {
                let dcap = zkquote.dcap_quote;
                Ok(ProtoEnclaveKeyInfo {
                    key_info: Some(enclave_key_info::KeyInfo::Zkdcap(ZkdcapEncalveKeyInfo {
                        dcap: Some(DcapEnclaveKeyInfo {
                            enclave_key_address: value.address.into(),
                            quote: dcap.raw,
                            fmspc: dcap.fmspc.to_vec(),
                            validity: dcap.validity.into(),
                            tcb_status: dcap.status,
                            advisory_ids: dcap.advisory_ids,
                            collateral: Some(dcap.collateral),
                        }),
                        zkp: Some(ZkvmProof {
                            proof: Some(match zkquote.zkp {
                                attestation_report::ZKVMProof::Risc0(proof) => {
                                    zkvm_proof::Proof::Risc0(Risc0ZkvmProof {
                                        image_id: proof.image_id.to_vec(),
                                        selector: proof.selector.to_vec(),
                                        seal: proof.seal,
                                        output: proof.output,
                                    })
                                }
                            }),
                        }),
                    })),
                })
            }
            None => Err(Error::unattested_enclave_key(value.address)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_report::{DCAPQuote, IASAttestationVerificationReport, IASSignedReport};
    use chrono::{DateTime, Duration, Utc};
    use lcp_types::proto::lcp::service::enclave::v1::{QvCollateral, Validity};
    use rand::RngCore;

    #[test]
    fn test_save() {
        for debug in [false, true] {
            let km = EnclaveKeyManager::new_in_memory().unwrap();
            let mrenclave = create_mrenclave();
            let sealed_ek = create_sealed_sk();
            let address = create_address();
            let report = create_report(mrenclave, address, debug);
            assert_eq!(km.all_keys().unwrap().len(), 0);
            km.save(sealed_ek, report, QEType::QE).unwrap();
            assert_eq!(km.all_keys().unwrap().len(), 1);
            assert_eq!(
                km.available_keys(mrenclave, debug, Some(RAType::IAS))
                    .unwrap()
                    .len(),
                0
            );
            let ki = km.load(address).unwrap();
            assert_eq!(ki.address, address);
            assert_eq!(ki.mrenclave, mrenclave);
            assert_eq!(ki.qe_type, QEType::QE);
            assert_eq!(ki.enclave_debug, debug);
            assert!(ki.ra_quote.is_none());

            let ias_report = create_ias_report(get_time(Duration::zero()));
            km.update_ra_quote(address, ias_report.into()).unwrap();
            let ki = km.load(address).unwrap();
            assert!(ki.ra_quote.is_some());
            // if RA quote already exists, should return an error
            assert!(km
                .update_ra_quote(
                    address,
                    create_ias_report(get_time(Duration::zero())).into()
                )
                .is_err());
        }
    }

    #[test]
    fn test_all_keys() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address1 = create_address();
        let report1 = create_report(mrenclave, address1, false);
        let address2 = create_address();
        let report2 = create_report(mrenclave, address2, false);
        let address3 = create_address();
        let report3 = create_report(mrenclave, address3, false);
        km.save(sealed_ek.clone(), report1, QEType::QE3).unwrap();
        km.save(sealed_ek.clone(), report2, QEType::QE3).unwrap();
        km.save(sealed_ek.clone(), report3, QEType::QE3).unwrap();
        let keys = km.all_keys().unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].address, address1);
        assert_eq!(keys[1].address, address2);
        assert_eq!(keys[2].address, address3);
    }

    #[test]
    fn test_available_keys() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address1 = create_address();
        let report1 = create_report(mrenclave, address1, false);
        let address2 = create_address();
        let report2 = create_report(mrenclave, address2, false);
        let address3 = create_address();
        let report3 = create_report(mrenclave, address3, false);
        km.save(sealed_ek.clone(), report1, QEType::QE3).unwrap();
        km.save(sealed_ek.clone(), report2, QEType::QE3).unwrap();
        km.save(sealed_ek.clone(), report3, QEType::QE3).unwrap();
        let keys = km.available_keys(mrenclave, false, None).unwrap();
        assert_eq!(keys.len(), 0);
        let dcap_quote = RAQuote::DCAP(create_dcap_quote(get_time2(Duration::days(30))));
        km.update_ra_quote(address1, dcap_quote).unwrap();
        let keys = km
            .available_keys(mrenclave, false, Some(RAType::DCAP))
            .unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].address, address1);
        let dcap_quote = RAQuote::DCAP(create_dcap_quote(get_time2(Duration::days(29))));
        km.update_ra_quote(address2, dcap_quote).unwrap();
        let keys = km
            .available_keys(mrenclave, false, Some(RAType::DCAP))
            .unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].address, address1);
        assert_eq!(keys[1].address, address2);
        let dcap_quote = RAQuote::DCAP(create_dcap_quote(get_time2(Duration::days(31))));
        km.update_ra_quote(address3, dcap_quote).unwrap();
        let keys = km
            .available_keys(mrenclave, false, Some(RAType::DCAP))
            .unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].address, address3);
        assert_eq!(keys[1].address, address1);
        assert_eq!(keys[2].address, address2);
    }

    #[test]
    fn test_key_expiration() {
        // Test for Unattested key
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address, false);
        km.save(sealed_ek, report, QEType::QE).unwrap();
        assert_eq!(km.all_keys().unwrap().len(), 1);
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::minutes(1))),
                PrunePolicy::ExpiredCreatedAt(60 + 1)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::minutes(1))),
                PrunePolicy::ExpiredCreatedAt(60)
            )
            .unwrap(),
            1
        );

        // Test for Attested key
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address, false);
        km.save(sealed_ek, report, QEType::QE).unwrap();
        let ias_report = create_ias_report(get_time(Duration::zero()));
        // valid_to = current + 30 days (IAS default validity)
        km.update_ra_quote(address, ias_report.into()).unwrap();
        assert_eq!(
            km.available_keys(mrenclave, false, Some(RAType::IAS))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(km.prune(None, PrunePolicy::ValidTo).unwrap(), 0);
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::days(30) - Duration::seconds(1))),
                PrunePolicy::ValidTo
            )
            .unwrap(),
            0
        );
        assert_eq!(
            km.prune(Some(get_time2(Duration::days(30))), PrunePolicy::ValidTo)
                .unwrap(),
            1
        );
        assert_eq!(
            km.available_keys(mrenclave, false, Some(RAType::IAS))
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn test_key_expiration_dcap() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address, false);
        km.save(sealed_ek, report, QEType::QE3).unwrap();
        assert_eq!(km.all_keys().unwrap().len(), 1);
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::minutes(1))),
                PrunePolicy::ExpiredCreatedAt(60 + 1)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::minutes(1))),
                PrunePolicy::ExpiredCreatedAt(60)
            )
            .unwrap(),
            1
        );

        // Test for Attested key
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address, false);
        km.save(sealed_ek, report, QEType::QE3).unwrap();
        let dcap_quote = RAQuote::DCAP(create_dcap_quote(get_time2(Duration::days(30))));
        km.update_ra_quote(address, dcap_quote).unwrap();
        assert_eq!(
            km.available_keys(mrenclave, false, Some(RAType::DCAP))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(km.prune(None, PrunePolicy::ValidTo).unwrap(), 0);
        assert_eq!(
            km.prune(
                Some(get_time2(Duration::days(30) - Duration::seconds(1))),
                PrunePolicy::ValidTo
            )
            .unwrap(),
            0
        );
        assert_eq!(
            km.prune(Some(get_time2(Duration::days(30))), PrunePolicy::ValidTo)
                .unwrap(),
            1
        );
        assert_eq!(
            km.available_keys(mrenclave, false, Some(RAType::DCAP))
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn test_key_info_conversion() {
        let km = EnclaveKeyManager::new_in_memory().unwrap();
        let mrenclave = create_mrenclave();
        let sealed_ek = create_sealed_sk();
        let address = create_address();
        let report = create_report(mrenclave, address, false);
        km.save(sealed_ek, report, QEType::QE).unwrap();
        let key_info = km.load(address).unwrap();
        assert!(ProtoEnclaveKeyInfo::try_from(key_info).is_err());
        let ias_report = create_ias_report(get_time(Duration::minutes(1)));
        km.update_ra_quote(address, ias_report.into()).unwrap();
        let key_info = km.load(address).unwrap();
        let res = ProtoEnclaveKeyInfo::try_from(key_info);
        assert!(res.is_ok(), "{:?}", res);
    }

    fn get_time(d: Duration) -> DateTime<Utc> {
        Utc::now().checked_add_signed(d).unwrap()
    }

    fn get_time2(d: Duration) -> Time {
        let tm = get_time(d);
        Time::from_unix_timestamp(tm.timestamp(), tm.timestamp_subsec_nanos()).unwrap()
    }

    fn create_mrenclave() -> Mrenclave {
        Mrenclave(rand::random())
    }

    fn create_sealed_sk() -> SealedEnclaveKey {
        let mut sealed_sk = [0u8; 592];
        rand::thread_rng().fill_bytes(&mut sealed_sk);
        SealedEnclaveKey::new_from_bytes(&sealed_sk).unwrap()
    }

    fn create_report(mrenclave: Mrenclave, ek_addr: Address, enclave_debug: bool) -> sgx_report_t {
        let mut report = sgx_report_t::default();
        report.body.mr_enclave = mrenclave.into();
        report.body.report_data = ReportData::new(ek_addr, None).into();
        report.body.attributes.flags = sgx_types::SGX_FLAGS_DEBUG * enclave_debug as u64;
        report
    }

    fn create_address() -> Address {
        let bz: [u8; 20] = rand::random();
        let addr = Address::try_from(bz.as_slice()).unwrap();
        addr
    }

    fn create_ias_report(timestamp: DateTime<Utc>) -> IASSignedReport {
        // TODO set correct quote body
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

    fn create_dcap_quote(valid_to: Time) -> DCAPQuote {
        DCAPQuote {
            raw: vec![0u8; 100],
            fmspc: [0u8; 6],
            validity: Validity {
                not_before: 0,
                not_after: valid_to.as_unix_timestamp_secs(),
            },
            status: "UpToDate".to_string(),
            advisory_ids: vec![],
            collateral: QvCollateral {
                tcb_info_json: "".to_string(),
                qe_identity_json: "".to_string(),
                sgx_intel_root_ca_der: vec![],
                sgx_tcb_signing_der: vec![],
                sgx_intel_root_ca_crl_der: vec![],
                sgx_pck_crl_der: vec![],
            },
        }
    }
}

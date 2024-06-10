use crate::errors::Error;
use crate::prelude::*;
use chrono::prelude::DateTime;
use core::fmt::{Debug, Display, Error as FmtError};
use crypto::Address;
use lcp_types::Time;
use serde::{Deserialize, Serialize};
use sgx_types::{metadata::metadata_t, sgx_measurement_t, sgx_quote_t, sgx_report_data_t};
use tendermint::Time as TmTime;

pub const REPORT_DATA_V1: u8 = 1;

/// ReportData is a 64-byte value that is embedded in the Quote
/// | version: 1 byte | enclave key: 20 bytes | operator: 20 bytes | nonce: 22 bytes |
#[derive(Debug, Clone, PartialEq)]
pub struct ReportData([u8; 64]);

impl ReportData {
    pub fn new(ek: Address, operator: Option<Address>) -> Self {
        let mut data: ReportData = Default::default();
        data.0[0] = REPORT_DATA_V1;
        data.0[1..21].copy_from_slice(ek.0.as_ref());
        if let Some(operator) = operator {
            data.0[21..41].copy_from_slice(operator.0.as_ref());
        }
        data
    }

    pub fn enclave_key(&self) -> Address {
        // Unwrap is safe because the size of the slice is 20
        Address::try_from(&self.0[1..21]).unwrap()
    }

    pub fn operator(&self) -> Address {
        // Unwrap is safe because the size of the slice is 20
        Address::try_from(&self.0[21..41]).unwrap()
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.0[0] != REPORT_DATA_V1 {
            return Err(Error::unexpected_report_data_version(
                REPORT_DATA_V1,
                self.0[0],
            ));
        }
        Ok(())
    }
}

impl Default for ReportData {
    fn default() -> Self {
        ReportData([0; 64])
    }
}

impl Display for ReportData {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), FmtError> {
        write!(f, "ReportData(0x{})", hex::encode(&self.0))
    }
}

impl From<ReportData> for sgx_report_data_t {
    fn from(data: ReportData) -> Self {
        sgx_report_data_t { d: data.0 }
    }
}

impl From<sgx_report_data_t> for ReportData {
    fn from(data: sgx_report_data_t) -> Self {
        ReportData(data.d)
    }
}

/// AttestationReport can be endorsed by either the Intel Attestation Service
/// using EPID or Data Center Attestation
/// Service (platform dependent) using ECDSA.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndorsedAttestationVerificationReport {
    /// Attestation report generated by the hardware
    pub avr: String,
    /// Singature of the report
    #[serde(with = "serde_base64")]
    pub signature: Vec<u8>,
    /// Certificate matching the signing key of the signature
    #[serde(with = "serde_base64")]
    pub signing_cert: Vec<u8>,
}

impl EndorsedAttestationVerificationReport {
    pub fn get_avr(&self) -> Result<AttestationVerificationReport, Error> {
        serde_json::from_slice(self.avr.as_bytes()).map_err(Error::serde_json)
    }
}

// AttestationVerificationReport represents Intel's Attestation Verification Report
// https://api.trustedservices.intel.com/documents/sgx-attestation-api-spec.pdf
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AttestationVerificationReport {
    pub id: String,
    pub timestamp: String,
    pub version: i64,
    #[serde(alias = "isvEnclaveQuoteStatus")]
    pub isv_enclave_quote_status: String,
    #[serde(alias = "isvEnclaveQuoteBody")]
    pub isv_enclave_quote_body: String,
    #[serde(alias = "revocationReason")]
    pub revocation_reason: Option<i64>,
    #[serde(alias = "pseManifestStatus")]
    pub pse_manifest_status: Option<i64>,
    #[serde(alias = "pseManifestHash")]
    pub pse_manifest_hash: Option<String>,
    #[serde(alias = "platformInfoBlob")]
    pub platform_info_blob: Option<String>,
    pub nonce: Option<String>,
    #[serde(alias = "epidPseudonym")]
    pub epid_pseudonym: Option<Vec<u8>>,
    #[serde(alias = "advisoryURL")]
    pub advisory_url: String,
    #[serde(alias = "advisoryIDs")]
    pub advisory_ids: Vec<String>,
}

impl AttestationVerificationReport {
    pub fn attestation_time(&self) -> Result<Time, Error> {
        let time_fixed = self.timestamp.clone() + "+0000";
        let dt = DateTime::parse_from_str(&time_fixed, "%Y-%m-%dT%H:%M:%S%.f%z").unwrap();

        Ok(
            TmTime::from_unix_timestamp(dt.timestamp(), dt.timestamp_subsec_nanos())
                .map_err(lcp_types::TimeError::tendermint)
                .map_err(Error::time_error)?
                .into(),
        )
    }

    pub fn parse_quote(&self) -> Result<Quote, Error> {
        if self.version != 4 {
            return Err(Error::unexpected_attestation_report_version(
                4,
                self.version,
            ));
        }

        let quote = base64::decode(&self.isv_enclave_quote_body).map_err(Error::base64)?;
        let sgx_quote: sgx_quote_t = unsafe { core::ptr::read(quote.as_ptr() as *const _) };
        Ok(Quote {
            raw: sgx_quote,
            status: self.isv_enclave_quote_status.clone(),
            attestation_time: self.attestation_time()?,
        })
    }

    #[cfg(feature = "std")]
    pub fn to_canonical_json(&self) -> Result<String, Error> {
        if self.version != 4 {
            return Err(Error::unexpected_attestation_report_version(
                4,
                self.version,
            ));
        }
        Ok(format!(
            "{}",
            serde_json::json!({
                "id": self.id,
                "timestamp": self.timestamp,
                "version": self.version,
                "advisoryURL": self.advisory_url,
                "advisoryIDs": self.advisory_ids,
                "isvEnclaveQuoteStatus": self.isv_enclave_quote_status,
                "platformInfoBlob": self.platform_info_blob,
                "isvEnclaveQuoteBody": self.isv_enclave_quote_body
            })
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    pub raw: sgx_quote_t,
    pub status: String,
    pub attestation_time: Time,
}

impl Quote {
    pub fn report_data(&self) -> ReportData {
        self.raw.report_body.report_data.into()
    }

    pub fn get_mrenclave(&self) -> sgx_measurement_t {
        self.raw.report_body.mr_enclave
    }

    pub fn match_metadata(&self, metadata: &metadata_t) -> Result<(), Error> {
        if self.raw.report_body.mr_enclave.m != metadata.enclave_css.body.enclave_hash.m {
            Err(Error::mrenclave_mismatch(
                self.raw.report_body.mr_enclave.m.into(),
                metadata.enclave_css.body.enclave_hash.m.into(),
            ))
        } else {
            Ok(())
        }
    }
}

mod serde_base64 {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = base64::encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        base64::decode(base64.as_bytes()).map_err(serde::de::Error::custom)
    }
}

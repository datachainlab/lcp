use crate::dcap::ZKDCAPQuote;
use crate::{dcap::DCAPQuote, errors::Error};
use crate::{prelude::*, IASSignedReport};
use core::fmt::{Debug, Display, Error as FmtError};
use core::str::FromStr;
use crypto::Address;
use lcp_types::Time;
use serde::{Deserialize, Serialize};
use sgx_types::{sgx_report_body_t, sgx_report_data_t};

/// The version of the report data format
pub const REPORT_DATA_V1: u8 = 1;

/// QEType is used to identify the type of the quoting enclave
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QEType {
    /// Quoting enclave (QE) for EPID quote
    #[default]
    QE,
    /// Quoting enclave (QE) for ECDSA quote
    QE3,
    /// Quoting enclave (QE) for ECDSA quote with simulation mode
    QE3SIM,
}

impl QEType {
    /// Returns the u32 representation of the QE type
    ///
    /// This is used for the EnclaveKeyManager to store the QE type in the DB
    ///
    /// | Type   | Value |
    /// |--------|-------|
    /// | QE     |   1   |
    /// | QE3    |   2   |
    /// | QE3SIM |   3   |
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::QE => 1,
            Self::QE3 => 2,
            Self::QE3SIM => 3,
        }
    }

    /// Returns the QE type from the u32 value
    pub fn from_u32(v: u32) -> Result<Self, Error> {
        match v {
            1 => Ok(Self::QE),
            2 => Ok(Self::QE3),
            3 => Ok(Self::QE3SIM),
            _ => Err(Error::invalid_qe_type(v)),
        }
    }
}

impl Display for QEType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QEType::QE => write!(f, "QE"),
            QEType::QE3 => write!(f, "QE3"),
            QEType::QE3SIM => write!(f, "QE3SIM"),
        }
    }
}

impl FromStr for QEType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "QE" => Ok(QEType::QE),
            "QE3" => Ok(QEType::QE3),
            "QE3SIM" => Ok(QEType::QE3SIM),
            _ => Err(anyhow::anyhow!("Invalid QE type: {}", s)),
        }
    }
}

/// RAType is used to identify the type of the remote attestation
#[derive(Debug, Serialize, Deserialize)]
pub enum RAType {
    /// Intel Attestation Service (IAS)
    IAS,
    /// Intel SGX Data Center Attestation Primitives (DCAP)
    DCAP,
    /// Remote attestation method based on a proof generated by a zkVM,
    /// which proves that DCAP quote verification was executed correctly
    ZKDCAPRisc0,
    /// Remote attestation method using a mock proof from the zkVM,
    /// intended for testing and development purposes.
    MockZKDCAPRisc0,
}

impl RAType {
    /// Returns the u32 representation of the RA type
    ///
    /// This is used for the EnclaveKeyManager to store the RA type in the DB
    ///
    /// | Type            | Value |
    /// |-----------------|-------|
    /// | IAS             |   1   |
    /// | DCAP            |   2   |
    /// | ZKDCAPRisc0     |   3   |
    /// | MockZKDCAPRisc0 |   4   |
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::IAS => 1,
            Self::DCAP => 2,
            Self::ZKDCAPRisc0 => 3,
            Self::MockZKDCAPRisc0 => 4,
        }
    }

    /// Returns the RA type from the u32 value
    pub fn from_u32(v: u32) -> Result<Self, Error> {
        match v {
            1 => Ok(Self::IAS),
            2 => Ok(Self::DCAP),
            3 => Ok(Self::ZKDCAPRisc0),
            4 => Ok(Self::MockZKDCAPRisc0),
            _ => Err(Error::invalid_ra_type(v)),
        }
    }
}

impl Display for RAType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), FmtError> {
        write!(
            f,
            "{}",
            match self {
                Self::IAS => "ias",
                Self::DCAP => "dcap",
                Self::ZKDCAPRisc0 => "zkdcap_risc0",
                Self::MockZKDCAPRisc0 => "mock_zkdcap_risc0",
            }
        )
    }
}

/// RAQuote is used to represent the remote attestation quote
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RAQuote {
    /// Intel Attestation Service (IAS) report
    IAS(IASSignedReport),
    /// Intel SGX Data Center Attestation Primitives (DCAP) quote
    DCAP(DCAPQuote),
    /// DCAP quote with zkVM proof
    ZKDCAP(ZKDCAPQuote),
}

impl RAQuote {
    /// Returns the RA type of the quote
    pub fn ra_type(&self) -> RAType {
        match self {
            RAQuote::IAS(_) => RAType::IAS,
            RAQuote::DCAP(_) => RAType::DCAP,
            RAQuote::ZKDCAP(quote) => {
                // currently only support Risc0
                if quote.is_mock_zkp() {
                    RAType::MockZKDCAPRisc0
                } else {
                    RAType::ZKDCAPRisc0
                }
            }
        }
    }

    /// Returns the attestation time of the quote
    pub fn attested_at(&self) -> Result<Time, Error> {
        match self {
            RAQuote::IAS(report) => report.get_avr()?.attestation_time(),
            RAQuote::DCAP(quote) => Ok(quote.attested_at),
            RAQuote::ZKDCAP(quote) => Ok(quote.dcap_quote.attested_at),
        }
    }

    /// Parses the RA quote from a JSON string
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }

    /// Converts the RA quote to a JSON string
    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }
}

impl From<IASSignedReport> for RAQuote {
    fn from(report: IASSignedReport) -> Self {
        RAQuote::IAS(report)
    }
}

impl From<DCAPQuote> for RAQuote {
    fn from(quote: DCAPQuote) -> Self {
        RAQuote::DCAP(quote)
    }
}

/// ReportData is a 64-byte value that is embedded in the Quote
///
/// The format of the report data is as follows:
/// | version: 1 byte | enclave key: 20 bytes | operator: 20 bytes | nonce: 22 bytes |
#[derive(Debug, Clone, PartialEq)]
pub struct ReportData(pub [u8; 64]);

impl ReportData {
    /// Creates a new report data
    pub fn new(ek: Address, operator: Option<Address>) -> Self {
        let mut data: ReportData = Default::default();
        data.0[0] = REPORT_DATA_V1;
        data.0[1..21].copy_from_slice(ek.0.as_ref());
        if let Some(operator) = operator {
            data.0[21..41].copy_from_slice(operator.0.as_ref());
        }
        data
    }

    /// Returns the enclave key from the report data
    ///
    /// CONTRACT: The report data must be validated before calling this function
    pub fn enclave_key(&self) -> Address {
        // Unwrap is safe because the size of the slice is 20
        Address::try_from(&self.0[1..21]).unwrap()
    }

    /// Returns the operator from the report data
    ///
    /// CONTRACT: The report data must be validated before calling this function
    pub fn operator(&self) -> Address {
        // Unwrap is safe because the size of the slice is 20
        Address::try_from(&self.0[21..41]).unwrap()
    }

    /// Validates the report data
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
        write!(f, "0x{}", hex::encode(self.0))
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

/// Checks if the enclave debug flag is enabled for the given report body
pub fn is_enclave_debug_enabled(report_body: &sgx_report_body_t) -> bool {
    report_body.attributes.flags & sgx_types::SGX_FLAGS_DEBUG != 0
}

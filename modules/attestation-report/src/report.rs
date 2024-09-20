use crate::{dcap::DCAPQuote, errors::Error};
use crate::{prelude::*, IASSignedReport};
use core::fmt::{Debug, Display, Error as FmtError};
use crypto::Address;
use lcp_types::Time;
use sgx_types::{metadata::metadata_t, sgx_measurement_t, sgx_quote_t, sgx_report_data_t};

pub const REPORT_DATA_V1: u8 = 1;

#[derive(Debug)]
pub enum VerifiableQuote {
    IAS(IASSignedReport),
    DCAP(DCAPQuote),
}

impl VerifiableQuote {
    pub fn attested_at(&self) -> Result<Time, Error> {
        match self {
            VerifiableQuote::IAS(report) => report.get_avr()?.attestation_time(),
            VerifiableQuote::DCAP(quote) => Ok(quote.attested_at),
        }
    }
}

impl From<IASSignedReport> for VerifiableQuote {
    fn from(report: IASSignedReport) -> Self {
        VerifiableQuote::IAS(report)
    }
}

impl From<DCAPQuote> for VerifiableQuote {
    fn from(quote: DCAPQuote) -> Self {
        VerifiableQuote::DCAP(quote)
    }
}

/// ReportData is a 64-byte value that is embedded in the Quote
/// | version: 1 byte | enclave key: 20 bytes | operator: 20 bytes | nonce: 22 bytes |
#[derive(Debug, Clone, PartialEq)]
pub struct ReportData([u8; 64]);

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

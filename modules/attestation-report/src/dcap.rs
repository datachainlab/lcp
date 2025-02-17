use crate::prelude::*;
use crate::Error;
use crate::RAQuote;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// DCAPQuote represents a quote, collateral, and supplementary data from the DCAP verification library
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DCAPQuote {
    /// Raw quote from the DCAP library
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub raw: Vec<u8>,
    /// Family Model Specific Platform Configuration (FMSPC) of the processor/platform
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub fmspc: [u8; 6],
    /// TCB status of the processor/platform
    pub tcb_status: String,
    /// Advisory IDs of the processor/platform
    pub advisory_ids: Vec<String>,
    /// Time when the quote was attested
    pub attested_at: Time,
    /// Collateral data used to verify the quote
    pub collateral: DcapCollateral,
}

impl DCAPQuote {
    /// Converts the quote to a RAQuote
    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }

    /// Parses the quote from a JSON string
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }

    /// Returns the report data from the quote
    #[cfg(feature = "std")]
    pub fn report_data(&self) -> Result<crate::ReportData, Error> {
        use dcap_quote_verifier::types::quotes::version_3::QuoteV3;
        let quote = QuoteV3::from_bytes(&self.raw).map_err(Error::dcap_quote_verifier)?;
        Ok(crate::ReportData(quote.isv_enclave_report.report_data))
    }
}

/// ZKVMProof represents a zkVM proof
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZKVMProof {
    Risc0(Risc0ZKVMProof),
}

impl ZKVMProof {
    /// Returns the output of dcap-quote-verifier program executed inside the zkVM
    pub fn output(&self) -> &[u8] {
        match self {
            ZKVMProof::Risc0(ref proof) => &proof.output,
        }
    }

    /// Returns true if the proof is a mock proof
    pub fn is_mock(&self) -> bool {
        match self {
            ZKVMProof::Risc0(ref proof) => proof.is_mock(),
        }
    }
}

/// Risc0ZKVMProof represents a zkVM proof for RISC Zero
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Risc0ZKVMProof {
    /// A small cryptographic identifier that indicates the method or boot image for zkVM execution
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub image_id: [u8; 32],
    /// selector indicating the zkVM version
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub selector: [u8; 4],
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    /// A Groth16 proof for the correct execution of the guest program.
    pub seal: Vec<u8>,
    /// The output of dcap-quote-verifier program executed inside the zkVM
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub output: Vec<u8>,
}

impl Risc0ZKVMProof {
    /// Returns true if the proof is a mock proof
    pub fn is_mock(&self) -> bool {
        self.selector == [0u8; 4]
    }
}

/// ZKDCAPQuote represents a DCAP quote with a zkVM proof
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZKDCAPQuote {
    /// DCAP quote
    pub dcap_quote: DCAPQuote,
    /// zkVM proof
    pub zkp: ZKVMProof,
}

impl From<ZKDCAPQuote> for RAQuote {
    fn from(quote: ZKDCAPQuote) -> Self {
        RAQuote::ZKDCAP(quote)
    }
}

impl ZKDCAPQuote {
    /// Creates a new ZKDCAPQuote
    pub fn new(dcap_quote: DCAPQuote, zkp: ZKVMProof) -> Self {
        ZKDCAPQuote { dcap_quote, zkp }
    }

    /// Returns true if the proof is a mock proof
    pub fn is_mock_zkp(&self) -> bool {
        self.zkp.is_mock()
    }

    /// Converts the quote to a JSON string
    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }

    /// Parses the quote from a JSON string
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }

    /// Returns the report data from the quote
    #[cfg(feature = "std")]
    pub fn report_data(&self) -> Result<crate::ReportData, Error> {
        self.dcap_quote.report_data()
    }

    /// Returns the commit corresponding to the zkVM proof
    #[cfg(feature = "std")]
    pub fn commit(&self) -> Result<dcap_quote_verifier::verifier::QuoteVerificationOutput, Error> {
        dcap_quote_verifier::verifier::QuoteVerificationOutput::from_bytes(self.zkp.output())
            .map_err(Error::dcap_quote_verifier)
    }
}

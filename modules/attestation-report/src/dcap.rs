use crate::prelude::*;
use crate::serde_base64;
use crate::Error;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DCAPQuote {
    #[serde(with = "serde_base64")]
    pub raw: Vec<u8>,
    pub fmspc: [u8; 6],
    pub tcb_status: String,
    pub advisory_ids: Vec<String>,
    pub attested_at: Time,
    pub collateral: DcapCollateral,
}

impl DCAPQuote {
    pub fn new(
        raw_quote: Vec<u8>,
        fmspc: [u8; 6],
        tcb_status: String,
        advisory_ids: Vec<String>,
        attested_at: Time,
        collateral: DcapCollateral,
    ) -> Self {
        DCAPQuote {
            raw: raw_quote,
            fmspc,
            tcb_status,
            advisory_ids,
            attested_at,
            collateral,
        }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }

    #[cfg(feature = "std")]
    pub fn report_data(&self) -> Result<crate::ReportData, Error> {
        use dcap_rs::types::quotes::version_3::QuoteV3;
        let quote = QuoteV3::from_bytes(&self.raw);
        Ok(crate::ReportData(quote.isv_enclave_report.report_data))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZKVMProof {
    Risc0(Risc0ZKVMProof),
}

impl ZKVMProof {
    pub fn commit(&self) -> &[u8] {
        match self {
            ZKVMProof::Risc0(ref proof) => &proof.commit,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Risc0ZKVMProof {
    pub seal: Vec<u8>,
    pub commit: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZKDCAPQuote {
    pub dcap_quote: DCAPQuote,
    pub zkp: ZKVMProof,
}

impl ZKDCAPQuote {
    pub fn new(dcap_quote: DCAPQuote, zkp: ZKVMProof) -> Self {
        ZKDCAPQuote { dcap_quote, zkp }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::serde_json)
    }

    #[cfg(feature = "std")]
    pub fn report_data(&self) -> Result<crate::ReportData, Error> {
        self.dcap_quote.report_data()
    }

    #[cfg(feature = "std")]
    pub fn commit(&self) -> DCAPVerifierCommit {
        DCAPVerifierCommit::from_bytes(self.zkp.commit())
    }
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq)]
pub struct DCAPVerifierCommit {
    pub attestation_time: u64,
    pub sgx_intel_root_ca_hash: [u8; 32],
    pub output: dcap_rs::types::VerifiedOutput,
}

#[cfg(feature = "std")]
impl DCAPVerifierCommit {
    pub fn new(
        attestation_time: u64,
        output: dcap_rs::types::VerifiedOutput,
        sgx_intel_root_ca: &[u8],
    ) -> Self {
        Self {
            attestation_time,
            sgx_intel_root_ca_hash: dcap_rs::utils::hash::keccak256sum(sgx_intel_root_ca),
            output,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = self.output.to_bytes();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.attestation_time.to_le_bytes());
        bytes.extend_from_slice(&self.sgx_intel_root_ca_hash);
        bytes.append(&mut output);
        bytes
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        let mut attestation_time = [0; 8];
        attestation_time.copy_from_slice(&slice[0..8]);
        let mut sgx_intel_root_ca_hash = [0; 32];
        sgx_intel_root_ca_hash.copy_from_slice(&slice[8..40]);
        let output = dcap_rs::types::VerifiedOutput::from_bytes(&slice[40..]);
        Self {
            attestation_time: u64::from_le_bytes(attestation_time),
            sgx_intel_root_ca_hash,
            output,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        dcap_rs::utils::hash::keccak256sum(&self.to_bytes())
    }
}

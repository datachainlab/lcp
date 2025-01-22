use crate::prelude::*;
use crate::Error;
use crate::RAQuote;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DCAPQuote {
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub raw: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
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

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Risc0ZKVMProof {
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub image_id: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub seal: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex<serde_with::formats::Lowercase>")]
    pub commit: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZKDCAPQuote {
    pub dcap_quote: DCAPQuote,
    pub zkp: ZKVMProof,
    pub mock: bool,
}

impl From<ZKDCAPQuote> for RAQuote {
    fn from(quote: ZKDCAPQuote) -> Self {
        RAQuote::ZKDCAP(quote)
    }
}

impl ZKDCAPQuote {
    pub fn new(dcap_quote: DCAPQuote, zkp: ZKVMProof, mock: bool) -> Self {
        ZKDCAPQuote {
            dcap_quote,
            zkp,
            mock,
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
        self.dcap_quote.report_data()
    }

    #[cfg(feature = "std")]
    pub fn commit(&self) -> dcap_rs::types::VerifiedOutput {
        dcap_rs::types::VerifiedOutput::from_bytes(self.zkp.commit())
    }
}

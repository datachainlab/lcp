use crate::prelude::*;
use crate::serde_base64;
use crate::Error;
use lcp_types::Time;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DCAPQuote {
    #[serde(with = "serde_base64")]
    pub raw: Vec<u8>,
    pub tcb_status: String,
    pub advisory_ids: Option<Vec<String>>,
    pub attested_at: Time,
}

impl DCAPQuote {
    pub fn new(
        raw_quote: Vec<u8>,
        tcb_status: String,
        advisory_ids: Option<Vec<String>>,
        attested_at: Time,
    ) -> Self {
        DCAPQuote {
            raw: raw_quote,
            tcb_status,
            advisory_ids,
            attested_at,
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

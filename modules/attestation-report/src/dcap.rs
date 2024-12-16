use crate::prelude::*;
use crate::serde_base64;
use crate::Error;
use crate::ReportData;
use lcp_types::Time;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DCAPQuote {
    #[serde(with = "serde_base64")]
    pub raw: Vec<u8>,
    pub tcb_status: TcbStatus,
    pub advisory_ids: Option<Vec<String>>,
    pub attested_at: Time,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TcbStatus {
    OK,
    TcbSwHardeningNeeded,
    TcbConfigurationAndSwHardeningNeeded,
    TcbConfigurationNeeded,
    TcbOutOfDate,
    TcbOutOfDateConfigurationNeeded,
    TcbRevoked,
    TcbUnrecognized,
}

impl TcbStatus {
    pub fn from_str(s: &str) -> Self {
        return match s {
            "UpToDate" => TcbStatus::OK,
            "SWHardeningNeeded" => TcbStatus::TcbSwHardeningNeeded,
            "ConfigurationAndSWHardeningNeeded" => TcbStatus::TcbConfigurationAndSwHardeningNeeded,
            "ConfigurationNeeded" => TcbStatus::TcbConfigurationNeeded,
            "OutOfDate" => TcbStatus::TcbOutOfDate,
            "OutOfDateConfigurationNeeded" => TcbStatus::TcbOutOfDateConfigurationNeeded,
            "Revoked" => TcbStatus::TcbRevoked,
            _ => TcbStatus::TcbUnrecognized,
        };
    }
}

impl ToString for TcbStatus {
    fn to_string(&self) -> String {
        return match self {
            TcbStatus::OK => "UpToDate".to_string(),
            TcbStatus::TcbSwHardeningNeeded => "SWHardeningNeeded".to_string(),
            TcbStatus::TcbConfigurationAndSwHardeningNeeded => {
                "ConfigurationAndSWHardeningNeeded".to_string()
            }
            TcbStatus::TcbConfigurationNeeded => "ConfigurationNeeded".to_string(),
            TcbStatus::TcbOutOfDate => "OutOfDate".to_string(),
            TcbStatus::TcbOutOfDateConfigurationNeeded => {
                "OutOfDateConfigurationNeeded".to_string()
            }
            TcbStatus::TcbRevoked => "Revoked".to_string(),
            TcbStatus::TcbUnrecognized => "Unrecognized".to_string(),
        };
    }
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
            tcb_status: TcbStatus::from_str(&tcb_status),
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
    pub fn report_data(&self) -> ReportData {
        use dcap_rs::types::quotes::version_3::QuoteV3;
        let quote = QuoteV3::from_bytes(&self.raw);
        ReportData(quote.isv_enclave_report.report_data)
    }
}

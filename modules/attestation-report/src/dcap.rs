use crate::prelude::*;
use crate::Error;
use lcp_types::Time;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DCAPQuote {
    pub raw: Vec<u8>,
    pub attested_at: Time,
}

impl DCAPQuote {
    pub fn new(raw_quote: Vec<u8>, attested_at: Time) -> Self {
        DCAPQuote {
            raw: raw_quote,
            attested_at,
        }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(Error::serde_json)
    }
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use lcp_types::Time;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationContext {
    pub current_timestamp: Time,
}

impl ValidationContext {
    pub fn new(current_timestamp: Time) -> Self {
        ValidationContext { current_timestamp }
    }
}

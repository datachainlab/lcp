#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, RlpEncodable, RlpDecodable)]
pub struct ValidationContext {
    current_timestamp: u64,
}

impl ValidationContext {
    pub fn to_vec(&self) -> Vec<u8> {
        rlp::encode(self).to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Self {
        rlp::decode(bz).unwrap()
    }
}

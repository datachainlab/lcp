use crate::commit::Commit;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SignedCommit {
    pub commit: Commit,
    pub pubkey: Vec<u8>,
    pub signature: Vec<u8>,
}

impl SignedCommit {
    pub fn new(commit: Commit, pubkey: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            commit,
            pubkey,
            signature,
        }
    }
}

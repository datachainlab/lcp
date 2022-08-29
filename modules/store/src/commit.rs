use crate::errors::{Result, StoreError as Error};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::vec::Vec;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitID([u8; 32]);

impl CommitID {
    pub fn new(bz: [u8; 32]) -> Self {
        Self(bz)
    }
}

impl From<[u8; 32]> for CommitID {
    fn from(bz: [u8; 32]) -> Self {
        Self(bz)
    }
}

impl Into<[u8; 32]> for CommitID {
    fn into(self) -> [u8; 32] {
        self.0
    }
}

pub type Revision = u64;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Commit {
    pub id: CommitID,
    pub revision: Revision,
}

impl Commit {
    pub fn new(id: CommitID, revision: Revision) -> Self {
        Self { id, revision }
    }

    pub fn as_sign_msg(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self).map_err(Error::BincodeError)
    }
}

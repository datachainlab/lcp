use crate::errors::{Result, StoreError as Error};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use anyhow::anyhow;
use log::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::format;
use std::string::ToString;
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

    pub fn as_sign_msg(&self) -> Result<[u8; 32]> {
        let bz = bincode::serialize(&self).map_err(Error::BincodeError)?;
        let mut hasher = Sha256::new();
        hasher.input(&bz);
        let mut msg: [u8; 32] = Default::default();
        msg.copy_from_slice(hasher.result().as_slice());
        Ok(msg)
    }
}

use crate::commit::Commit;
use crate::errors::Result;
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

pub trait CommitVerifier {
    fn get_pubkey(&self) -> Vec<u8>;
    fn verify(&self, msg: &[u8; 32], signature: &[u8]) -> Result<()>;
}

pub trait CommitSigner {
    fn sign_hash(&self, msg: &[u8; 32]) -> Result<Vec<u8>>;
    fn use_verifier(&self, f: &mut dyn FnMut(&dyn CommitVerifier));
}

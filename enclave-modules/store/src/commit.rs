use crate::{Result, StoreError as Error};
use anyhow::anyhow;
use enclave_crypto::EnclavePublicKey;
use enclave_utils::storage::{read_from_untrusted, write_to_untrusted};
use log::*;
use serde::{Deserialize, Serialize};
use settings::{COMMIT_ID_DIR, LAST_COMMIT_SEQUENCE};
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

pub type Sequence = u64;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Commit {
    pub id: CommitID,
    pub seq: Sequence,
}

impl Commit {
    pub fn new(id: CommitID, seq: Sequence) -> Self {
        Self { id, seq }
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

    pub fn verify(&self, epk: &EnclavePublicKey) -> Result<()> {
        let pubkey = EnclavePublicKey::from_bytes(&self.pubkey).map_err(Error::CryptoError)?;
        if epk != &pubkey {
            return Err(Error::OtherError(anyhow!(
                "public key mismatch: {:?} != {:?}",
                epk,
                pubkey
            )));
        }
        pubkey
            .verify(&self.commit.as_sign_msg()?, &self.signature)
            .map_err(Error::CryptoError)
    }
}

pub fn get_last_commit_seq() -> Result<Option<Sequence>> {
    let bz = match read_from_untrusted(&LAST_COMMIT_SEQUENCE) {
        Ok(bz) if bz.len() == 8 => bz,
        Ok(bz) => return Err(Error::OtherError(anyhow!("unexpected size: {}", bz.len()))),
        // TODO distinguish file errors
        Err(e) => {
            warn!("failed to read_from_untrusted: {}", e);
            return Ok(None);
        }
    };
    let mut seq: [u8; 8] = Default::default();
    seq.copy_from_slice(&bz);
    Ok(Some(Sequence::from_be_bytes(seq)))
}

pub fn get_last_commit() -> Result<Option<SignedCommit>> {
    let seq = match get_last_commit_seq()? {
        Some(seq) => seq,
        None => return Ok(None),
    };
    let commit_id_path = format!("{}/{}", COMMIT_ID_DIR.to_string(), seq);
    let bz = read_from_untrusted(&commit_id_path).map_err(Error::SGXError)?;
    Ok(Some(
        bincode::deserialize(&bz).map_err(Error::BincodeError)?,
    ))
}

pub fn save_commit(scommit: &SignedCommit) -> Result<()> {
    let commit_id_path = format!("{}/{}", COMMIT_ID_DIR.to_string(), scommit.commit.seq);
    let bz = bincode::serialize(scommit).map_err(Error::BincodeError)?;
    write_to_untrusted(&bz, &commit_id_path).map_err(Error::SGXError)?;
    write_to_untrusted(&scommit.commit.seq.to_be_bytes(), &LAST_COMMIT_SEQUENCE)
        .map_err(Error::SGXError)?;
    Ok(())
}

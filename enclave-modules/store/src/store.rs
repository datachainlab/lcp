use crate::{Result, StoreError as Error};
use anyhow::anyhow;
use enclave_crypto::{EnclaveKey, EnclavePublicKey};
use std::format;
use std::vec::Vec;

use crate::{commit::SignedCommit, Commit, CommitID, Sequence};

pub trait Store: KVStore + CommitStore + LoadableStore {}

pub trait KVStore {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>);
    fn get(&self, k: &[u8]) -> Option<Vec<u8>>;
}

pub trait CommitStore {
    fn commit(&mut self) -> Result<Commit>;
    fn commit_and_sign(&mut self, signer: Option<&EnclaveKey>) -> Result<SignedCommit>;
    fn rollback(&mut self);
    fn clear(&mut self) -> Result<()>;

    fn calculate_commit_id(&self) -> Result<CommitID>;

    fn get_current_sequence(&self) -> Result<Sequence>;
    fn get_last_commit(&self) -> Result<Option<SignedCommit>>;
}

pub trait LoadableStore: CommitStore {
    fn load_state(&mut self, expected_signer: Option<&EnclavePublicKey>) -> Result<()> {
        let expected_signer = match expected_signer {
            Some(v) => v,
            None => {
                return Err(Error::OtherError(anyhow!(
                    "expected_signer must be specified"
                )));
            }
        };

        match self.get_last_commit()? {
            Some(sc) => {
                let seq = self.get_current_sequence()?;
                if sc.commit.seq + 1 != seq {
                    return Err(Error::OtherError(anyhow!(
                        "commit sequence mismatch: {} != {}",
                        sc.commit.seq + 1,
                        seq
                    )));
                }
                let commit_id = self.calculate_commit_id()?;
                if sc.commit.id != commit_id {
                    return Err(Error::OtherError(anyhow!(
                        "commit id mismatch: {:?} != {:?}",
                        sc.commit.id,
                        commit_id
                    )));
                }
                sc.verify(expected_signer)
            }
            None => self.clear(),
        }
    }
}

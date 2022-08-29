#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{errors::Result, Commit, CommitID, Revision, SignedCommit, StoreError};
use crypto::{Signer, Verifier};
use std::vec::Vec;

pub trait Store: KVStore + VerifiablePersistentStore {}

pub trait KVStore {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>);
    fn get(&self, k: &[u8]) -> Option<Vec<u8>>;
}

pub trait CommitStore {
    fn commit(&mut self) -> Result<Commit>;
    fn rollback(&mut self);
    fn clear(&mut self) -> Result<()>;

    fn calculate_commit_id(&self) -> Result<CommitID>;
    fn current_revision(&self) -> Result<Revision>;
}

pub trait PersistentStore<T> {
    fn load(&mut self) -> Result<Option<T>>;
    fn save(&mut self, sc: &T) -> Result<()>;
}

pub trait VerifiablePersistentStore: CommitStore + PersistentStore<SignedCommit> {
    fn load_and_verify(&mut self, verifier: &dyn Verifier) -> Result<()> {
        match self.load()? {
            Some(sc) => verifier
                .verify(&sc.commit.as_sign_msg()?, &sc.signature)
                .map_err(StoreError::CryptoError),
            None => Ok(()),
        }
    }

    fn commit_and_sign(&mut self, signer: &dyn Signer) -> Result<SignedCommit> {
        let commit = self.commit()?;
        let sig = signer
            .sign(&commit.as_sign_msg()?)
            .map_err(StoreError::CryptoError)?;
        let mut pubkey = Default::default();
        signer.use_verifier(&mut |verifier: &dyn Verifier| {
            pubkey = verifier.get_pubkey();
        });
        let sc = SignedCommit::new(commit, pubkey, sig);
        self.save(&sc)?;
        Ok(sc)
    }
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    errors::Result, Commit, CommitID, CommitSigner, CommitVerifier, Revision, SignedCommit,
};
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
    fn load_and_verify(&mut self, verifier: &dyn CommitVerifier) -> Result<()> {
        match self.load()? {
            Some(sc) => verifier.verify(&sc.commit.as_sign_msg()?, &sc.signature),
            None => Ok(()),
        }
    }

    fn commit_and_sign(&mut self, signer: &dyn CommitSigner) -> Result<SignedCommit> {
        let commit = self.commit()?;
        let sig = signer.sign_hash(&commit.as_sign_msg()?)?;
        let mut pubkey = Default::default();
        signer.use_verifier(&mut |verifier: &dyn CommitVerifier| {
            pubkey = verifier.get_pubkey();
        });
        let sc = SignedCommit::new(commit, pubkey, sig);
        self.save(&sc)?;
        Ok(sc)
    }
}

use crate::errors::Result;
use crate::memory::MemStore;
use crate::{sgx_reexport_prelude::*, CommitSigner, CommitVerifier};
use crate::{
    Commit, CommitID, CommitStore, KVStore, PersistentStore, Revision, SignedCommit,
    VerifiablePersistentStore, Store,
};
use std::sync::Arc;
use std::sync::SgxRwLock;
use std::vec::Vec;

impl<T> KVStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.write().unwrap().set(k, v)
    }

    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        self.read().unwrap().get(k)
    }
}

impl<T> CommitStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn commit(&mut self) -> Result<Commit> {
        self.write().unwrap().commit()
    }

    fn rollback(&mut self) {
        self.write().unwrap().rollback()
    }

    fn clear(&mut self) -> Result<()> {
        self.write().unwrap().clear()
    }

    fn calculate_commit_id(&self) -> Result<CommitID> {
        self.read().unwrap().calculate_commit_id()
    }

    fn current_revision(&self) -> Result<Revision> {
        self.read().unwrap().current_revision()
    }
}

impl<T> PersistentStore<SignedCommit> for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn load(&mut self) -> Result<Option<SignedCommit>> {
        self.write().unwrap().load()
    }

    fn save(&mut self, commit: &SignedCommit) -> Result<()> {
        self.write().unwrap().save(commit)
    }
}

impl<T> VerifiablePersistentStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn load_and_verify(&mut self, verifier: &dyn CommitVerifier) -> Result<()> {
        self.write().unwrap().load_and_verify(verifier)
    }

    fn commit_and_sign(&mut self, signer: &dyn CommitSigner) -> Result<SignedCommit> {
        self.write().unwrap().commit_and_sign(signer)
    }
}

impl Store for Arc<SgxRwLock<MemStore>> {}

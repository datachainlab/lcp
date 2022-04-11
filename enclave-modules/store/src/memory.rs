use crate::commit::{get_last_commit, save_commit, SignedCommit};
use crate::store::{CommitStore, KVStore, LoadableStore, Store};
use crate::Result;
use crate::{Commit, CommitID, Sequence};
use enclave_crypto::{EnclaveKey, EnclavePublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::SgxRwLock;
use std::vec::Vec;

// MemStore is only available for testing purposes
#[derive(Default)]
pub struct MemStore {
    revision: u64,
    committed: MemMap,
    cached: MemMap,
}

#[derive(Default, Serialize, Deserialize)]
struct MemMap(#[serde(with = "hash_map_bytes")] HashMap<Vec<u8>, Vec<u8>>);

impl MemStore {
    pub fn new() -> Self {
        let mut store = MemStore::default();
        store.clear().unwrap();
        store
    }
}

impl Store for MemStore {}

impl Store for Arc<SgxRwLock<MemStore>> {}

impl KVStore for MemStore {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.cached.0.insert(k, v);
    }
    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        match self.cached.0.get(k) {
            Some(v) => Some(v.clone()),
            None => match self.committed.0.get(k) {
                Some(v) => Some(v.clone()),
                None => None,
            },
        }
    }
}

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

impl CommitStore for MemStore {
    fn commit(&mut self) -> Result<Commit> {
        self.committed.0.extend(self.cached.0.clone());
        self.cached.0.clear();

        let commit = Commit::new(self.calculate_commit_id()?, self.revision);
        self.revision += 1;
        Ok(commit)
    }

    fn commit_and_sign(&mut self, signer: Option<&EnclaveKey>) -> Result<SignedCommit> {
        let commit = self.commit()?;
        let signer = signer.unwrap();
        let sig = signer.sign_hash(commit.as_sign_msg()?).unwrap();
        let sc = SignedCommit::new(commit, signer.get_pubkey().as_bytes().to_vec(), sig);
        save_commit(&sc)?;
        Ok(sc)
    }

    fn rollback(&mut self) {
        self.cached.0.clear()
    }

    fn clear(&mut self) -> Result<()> {
        self.revision = 1;
        self.committed.0.clear();
        self.cached.0.clear();
        Ok(())
    }

    fn calculate_commit_id(&self) -> Result<CommitID> {
        let s = serde_json::to_string(&self.committed).unwrap();

        let mut id: [u8; 32] = Default::default();
        let mut hasher = Sha256::new();
        hasher.input(s.as_bytes());
        let h = hasher.result();
        id.copy_from_slice(&h);
        Ok(id.into())
    }

    fn get_current_sequence(&self) -> Result<Sequence> {
        Ok(self.revision)
    }

    fn get_last_commit(&self) -> Result<Option<SignedCommit>> {
        get_last_commit()
    }
}

impl<T> CommitStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn commit(&mut self) -> Result<Commit> {
        self.write().unwrap().commit()
    }

    fn commit_and_sign(&mut self, signer: Option<&EnclaveKey>) -> Result<SignedCommit> {
        self.write().unwrap().commit_and_sign(signer)
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

    fn get_current_sequence(&self) -> Result<Sequence> {
        self.read().unwrap().get_current_sequence()
    }

    fn get_last_commit(&self) -> Result<Option<SignedCommit>> {
        self.read().unwrap().get_last_commit()
    }
}

impl LoadableStore for MemStore {}

impl<T> LoadableStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn load_state(&mut self, expected_signer: Option<&EnclavePublicKey>) -> Result<()> {
        self.write().unwrap().load_state(expected_signer)
    }
}

mod hash_map_bytes {
    use super::Vec;
    use serde::{Deserializer, Serializer};

    type HashMapBytes = std::collections::HashMap<Vec<u8>, Vec<u8>>;

    pub(super) fn serialize<S: Serializer>(attr: &HashMapBytes, ser: S) -> Result<S::Ok, S::Error> {
        let attr: Vec<_> = attr.iter().collect();
        serde::Serialize::serialize(&attr, ser)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(des: D) -> Result<HashMapBytes, D::Error> {
        let attr: Vec<_> = serde::Deserialize::deserialize(des)?;
        Ok(attr.into_iter().collect())
    }
}

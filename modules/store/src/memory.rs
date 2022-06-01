use crate::errors::Result;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    Commit, CommitID, CommitStore, KVStore, PersistentStore, Revision, SignedCommit, Store,
    VerifiablePersistentStore,
};
use log::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ops::Deref;
use std::vec::Vec;

// MemStore is only available for testing purposes
#[derive(Default, Debug)]
pub struct MemStore {
    pub revision: u64,
    pub committed: MemMap,
    pub cached: MemMap,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MemMap(#[serde(with = "hash_map_bytes")] HashMap<Vec<u8>, Vec<u8>>);

impl MemStore {
    pub fn new() -> Self {
        let mut store = MemStore::default();
        store.clear().unwrap();
        store
    }
}

impl Deref for MemMap {
    type Target = HashMap<Vec<u8>, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Store for MemStore {}

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

impl CommitStore for MemStore {
    fn commit(&mut self) -> Result<Commit> {
        self.committed.0.extend(self.cached.0.clone());
        self.cached.0.clear();

        let commit = Commit::new(self.calculate_commit_id()?, self.revision);
        self.revision += 1;
        Ok(commit)
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

    fn current_revision(&self) -> Result<Revision> {
        Ok(self.revision)
    }
}

impl PersistentStore<SignedCommit> for MemStore {
    fn load(&mut self) -> Result<Option<SignedCommit>> {
        info!("load() is called");
        Ok(None)
    }

    fn save(&mut self, sc: &SignedCommit) -> Result<()> {
        info!("save() is called with {:?}", sc);
        Ok(())
    }
}

impl VerifiablePersistentStore for MemStore {}

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

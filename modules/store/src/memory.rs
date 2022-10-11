use crate::{prelude::*, Store};
use crate::{CommitStore, Error, KVStore};
use core::ops::Deref;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sgx")]
use sgx_tstd::collections::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

// MemStore is only available for testing purposes
#[derive(Default, Debug)]
pub struct MemStore {
    pub committed: MemMap,
    pub cached: MemMap,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MemMap(#[serde(with = "hash_map_bytes")] HashMap<Vec<u8>, Vec<u8>>);

impl Deref for MemMap {
    type Target = HashMap<Vec<u8>, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    fn commit(&mut self) -> Result<(), Error> {
        self.committed.0.extend(self.cached.0.clone());
        self.cached.0.clear();
        Ok(())
    }

    fn rollback(&mut self) {
        self.cached.0.clear()
    }
}

impl Store for MemStore {}

mod hash_map_bytes {
    use super::*;
    use serde::{Deserializer, Serializer};

    type HashMapBytes = HashMap<Vec<u8>, Vec<u8>>;

    pub(super) fn serialize<S: Serializer>(attr: &HashMapBytes, ser: S) -> Result<S::Ok, S::Error> {
        let attr: Vec<_> = attr.iter().collect();
        serde::Serialize::serialize(&attr, ser)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(des: D) -> Result<HashMapBytes, D::Error> {
        let attr: Vec<_> = serde::Deserialize::deserialize(des)?;
        Ok(attr.into_iter().collect())
    }
}

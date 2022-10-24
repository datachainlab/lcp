use crate::prelude::*;
use crate::store::TxId;
use crate::{CommitStore, Error, KVStore, Result};
#[cfg(feature = "sgx")]
use sgx_tstd::collections::{HashMap, HashSet};
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

// MemStore is only available for testing purposes
#[derive(Default, Debug)]
pub struct MemStore {
    latest_tx_id: TxId,
    uncommitted_data: HashMap<TxId, HashMap<Vec<u8>, Vec<u8>>>,
    uncommitted_tombstones: HashMap<TxId, HashSet<Vec<u8>>>,
    committed_data: HashMap<Vec<u8>, Vec<u8>>,
}

impl KVStore for MemStore {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.committed_data.get(key).map(|v| v.to_vec())
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.committed_data.insert(key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        self.committed_data.remove(key);
    }
}

impl CommitStore for MemStore {
    fn begin(&mut self) -> Result<TxId> {
        self.latest_tx_id.safe_incr()?;
        self.uncommitted_data
            .insert(self.latest_tx_id, Default::default());
        self.uncommitted_tombstones
            .insert(self.latest_tx_id, Default::default());
        Ok(self.latest_tx_id)
    }

    fn commit(&mut self, tx_id: TxId) -> Result<()> {
        let uncommitted_data = self
            .uncommitted_data
            .remove(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?;

        let uncommitted_tombstones = self
            .uncommitted_tombstones
            .remove(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?;

        self.committed_data.extend(uncommitted_data);
        for k in uncommitted_tombstones {
            self.committed_data.remove(&k);
        }
        Ok(())
    }

    fn rollback(&mut self, tx_id: TxId) {
        self.uncommitted_data.remove(&tx_id).unwrap();
        self.uncommitted_tombstones.remove(&tx_id).unwrap();
    }

    fn tx_get(&self, tx_id: TxId, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if self
            .uncommitted_tombstones
            .get(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?
            .contains(key)
        {
            return Ok(None);
        }
        let uncommitted_data = self
            .uncommitted_data
            .get(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?;
        match uncommitted_data.get(key) {
            Some(v) => Ok(Some(v.clone())),
            None => match self.committed_data.get(key) {
                Some(v) => Ok(Some(v.clone())),
                None => Ok(None),
            },
        }
    }

    fn tx_set(&mut self, tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.uncommitted_tombstones
            .get_mut(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?
            .remove(&key);
        self.uncommitted_data
            .get_mut(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?
            .insert(key, value);
        Ok(())
    }

    fn tx_remove(&mut self, tx_id: TxId, key: &[u8]) -> Result<()> {
        self.uncommitted_tombstones
            .get_mut(&tx_id)
            .ok_or_else(|| Error::tx_id_not_found(tx_id))?
            .insert(key.into());
        Ok(())
    }
}

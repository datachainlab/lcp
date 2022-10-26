use crate::prelude::*;
use crate::store::TxId;
use crate::transaction::{CommitStore, CreatedTx, Tx, TxStore};
use crate::{KVStore, Result};
use std::collections::HashMap;

// MemStore is only available for testing purposes
#[derive(Default, Debug)]
pub struct MemStore {
    running_tx_exists: bool,
    latest_tx_id: TxId,
    uncommitted_data: HashMap<Vec<u8>, Option<Vec<u8>>>,
    committed_data: HashMap<Vec<u8>, Vec<u8>>,
}

impl KVStore for MemStore {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if self.running_tx_exists {
            match self.uncommitted_data.get(key) {
                Some(v) => v.clone(),
                None => self.committed_data.get(key).map(|v| v.to_vec()),
            }
        } else {
            self.committed_data.get(key).map(|v| v.to_vec())
        }
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if self.running_tx_exists {
            self.uncommitted_data.insert(key, Some(value));
        } else {
            self.committed_data.insert(key, value);
        }
    }

    fn remove(&mut self, key: &[u8]) {
        if self.running_tx_exists {
            self.uncommitted_data.insert(key.to_vec(), None);
        } else {
            self.committed_data.remove(key);
        }
    }
}

impl TxStore for MemStore {
    fn run_in_tx<T>(&self, _tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T> {
        Ok(f(self))
    }

    fn run_in_mut_tx<T>(
        &mut self,
        _tx_id: TxId,
        f: impl FnOnce(&mut dyn KVStore) -> T,
    ) -> Result<T> {
        Ok(f(self))
    }
}

impl CommitStore for MemStore {
    type Tx = MemTx;

    fn create_transaction(
        &mut self,
        _update_key: Option<crate::transaction::UpdateKey>,
    ) -> Result<Self::Tx> {
        self.latest_tx_id.safe_incr()?;
        Ok(MemTx(self.latest_tx_id))
    }

    fn begin(&mut self, _tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        assert!(!self.running_tx_exists);
        self.running_tx_exists = true;
        Ok(())
    }

    fn commit(&mut self, _tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        assert!(self.running_tx_exists);
        self.running_tx_exists = false;
        let data = HashMap::<Vec<u8>, Option<Vec<u8>>>::default();
        let uncommitted_data = std::mem::replace(&mut self.uncommitted_data, data);
        for it in uncommitted_data {
            match it.1 {
                Some(v) => self.committed_data.insert(it.0, v),
                None => self.committed_data.remove(&it.0),
            };
        }
        Ok(())
    }

    fn rollback(&mut self, _tx: <Self::Tx as CreatedTx>::PreparedTx) {
        assert!(self.running_tx_exists);
        self.running_tx_exists = false;
        self.uncommitted_data.clear();
    }
}

pub struct MemTx(TxId);

impl Tx for MemTx {
    fn get_id(&self) -> TxId {
        self.0
    }
}

impl CreatedTx for MemTx {
    type PreparedTx = MemTx;

    fn prepare(self) -> Result<Self::PreparedTx> {
        Ok(self)
    }
}

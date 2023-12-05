use crate::prelude::*;
use crate::store::TxId;
use crate::transaction::{CommitStore, CreatedTx, Tx, TxAccessor};
use crate::{KVStore, Result};
use std::collections::HashMap;
use std::sync::Mutex;

// MemStore is only available for testing purposes
#[derive(Default, Debug)]
pub struct MemStore(Mutex<InnerMemStore>);

impl KVStore for MemStore {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(key)
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.0.lock().unwrap().set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.lock().unwrap().remove(key)
    }
}

impl TxAccessor for MemStore {
    fn run_in_tx<T>(&self, tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T> {
        self.0.lock().unwrap().run_in_tx(tx_id, f)
    }

    fn run_in_mut_tx<T>(
        &mut self,
        tx_id: TxId,
        f: impl FnOnce(&mut dyn KVStore) -> T,
    ) -> Result<T> {
        self.0.lock().unwrap().run_in_mut_tx(tx_id, f)
    }
}

impl CommitStore for MemStore {
    type Tx = MemTx;

    fn create_transaction(
        &mut self,
        _update_key: Option<crate::transaction::UpdateKey>,
    ) -> Result<Self::Tx> {
        self.0.lock().unwrap().create_transaction(_update_key)
    }

    fn begin(&mut self, tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.0.lock().unwrap().begin(tx)
    }

    fn commit(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.0.lock().unwrap().commit(tx)
    }

    fn rollback(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) {
        self.0.lock().unwrap().rollback(tx)
    }
}

#[derive(Default, Debug)]
pub struct InnerMemStore {
    running_tx_exists: bool,
    latest_tx_id: TxId,
    uncommitted_data: HashMap<Vec<u8>, Option<Vec<u8>>>,
    committed_data: HashMap<Vec<u8>, Vec<u8>>,
}

impl KVStore for InnerMemStore {
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

impl TxAccessor for InnerMemStore {
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

impl CommitStore for InnerMemStore {
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

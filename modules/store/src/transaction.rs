use crate::host::HostStoreAccessor;
use crate::prelude::*;
use crate::{KVStore, Result, TxId};

pub type UpdateKey = String;

pub trait Tx {
    fn get_id(&self) -> TxId;
}

pub trait CreatedTx: Tx {
    type PreparedTx: Tx;

    fn prepare(self) -> Result<Self::PreparedTx>;
}

pub trait CommitStore {
    type Tx: CreatedTx;

    fn create_transaction(&mut self, update_key: Option<UpdateKey>) -> Result<Self::Tx>;

    fn begin(&mut self, tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()>;
    fn commit(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()>;
    fn rollback(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx);
}

pub trait TxStore {
    fn run_in_tx<T>(&self, tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T>;

    fn run_in_mut_tx<T>(&mut self, tx_id: TxId, f: impl FnOnce(&mut dyn KVStore) -> T)
        -> Result<T>;

    fn tx_get(&self, tx_id: TxId, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.run_in_tx(tx_id, |tx| tx.get(key))
    }
    fn tx_set(&mut self, tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.run_in_mut_tx(tx_id, |tx| tx.set(key, value))
    }
    fn tx_remove(&mut self, tx_id: TxId, key: &[u8]) -> Result<()> {
        self.run_in_mut_tx(tx_id, |tx| tx.remove(key))
    }
}

pub trait TxManager<S: CommitStore>: HostStoreAccessor<S> {
    fn begin_tx(&self, update_key: Option<UpdateKey>) -> Result<<S::Tx as CreatedTx>::PreparedTx> {
        let tx = self.use_mut_store(|store| store.create_transaction(update_key))?;
        let tx = tx.prepare()?;
        self.use_mut_store(|store| store.begin(&tx))?;
        Ok(tx)
    }
    fn commit_tx(&self, tx: <S::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.use_mut_store(|store| store.commit(tx))
    }
    fn rollback_tx(&self, tx: <S::Tx as CreatedTx>::PreparedTx) {
        self.use_mut_store(|store| store.rollback(tx));
    }
}

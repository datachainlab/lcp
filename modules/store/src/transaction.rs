use crate::prelude::*;
use crate::{KVStore, Result, TxId};

/// `UpdateKey` is a hint to the store to control concurrent transactions
pub type UpdateKey = String;

/// `Tx` is a handle for a transaction
pub trait Tx {
    fn get_id(&self) -> TxId;
}

/// `CreatedTx` is a handle to a transaction that cannot be begun yet
pub trait CreatedTx: Tx {
    /// `PreparedTx` is a handle to a transaction that has been begun or can be begun
    type PreparedTx: Tx;

    /// `prepare` consumes the self to generate `PreparedTx`
    fn prepare(self) -> Result<Self::PreparedTx>;
}

/// `CommitStore` is a store that supports transactions
pub trait CommitStore {
    type Tx: CreatedTx;

    /// `create_transaction` creates a transaction with a given `update_key`
    /// if `update_key` is Some(k), it is desired that the store controls transactions that reference the same `k` in concurrent
    /// if `update_key` is None, it is desired that the store controls a transaction as read-only
    fn create_transaction(&mut self, update_key: Option<UpdateKey>) -> Result<Self::Tx>;

    /// `begin` begins the transaction
    fn begin(&mut self, tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()>;

    /// `commit` consume the transaction handle to commit the changes
    fn commit(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()>;

    /// `rollback` consume the transaction handle to rollback the changes
    fn rollback(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx);
}

/// `TxAccessor` is an accessor to a transaction that has already begun
pub trait TxAccessor {
    /// `run_in_tx` run a given function in specified transaction
    fn run_in_tx<T>(&self, tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T>;

    /// `run_in_mut_tx` run a given function in specified transaction
    fn run_in_mut_tx<T>(&mut self, tx_id: TxId, f: impl FnOnce(&mut dyn KVStore) -> T)
        -> Result<T>;

    /// `tx_get` returns a value corresponding to `key` in a specified transaction
    fn tx_get(&self, tx_id: TxId, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.run_in_tx(tx_id, |tx| tx.get(key))
    }

    /// `tx_set` sets key-value pair in a specified transaction
    fn tx_set(&mut self, tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.run_in_mut_tx(tx_id, |tx| tx.set(key, value))
    }

    /// `tx_remove` removes key-value pair corresponding to `key` in a specified transaction
    fn tx_remove(&mut self, tx_id: TxId, key: &[u8]) -> Result<()> {
        self.run_in_mut_tx(tx_id, |tx| tx.remove(key))
    }
}

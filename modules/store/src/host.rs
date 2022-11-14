use crate::memory::MemStore;
use crate::prelude::*;
use crate::transaction::{CommitStore, TxAccessor};
use crate::{KVStore, Result, TxId};

/// `HostStore` defines store implementations on host
pub enum HostStore {
    #[cfg(feature = "rocksdbstore")]
    RocksDB(crate::rocksdb::RocksDBStore),
    Memory(crate::memory::MemStore),
}

/// `IntoCommitStore` converts self into CommitStore
pub trait IntoCommitStore<S: CommitStore> {
    /// `apply` applies `f` to CommitStore
    fn apply<T>(&mut self, f: impl FnOnce(&mut S) -> T) -> T;
}

#[cfg(feature = "rocksdbstore")]
impl IntoCommitStore<crate::rocksdb::RocksDBStore> for HostStore {
    fn apply<T>(&mut self, f: impl FnOnce(&mut crate::rocksdb::RocksDBStore) -> T) -> T {
        match self {
            HostStore::RocksDB(store) => f(store),
            _ => unreachable!(),
        }
    }
}

impl IntoCommitStore<MemStore> for HostStore {
    fn apply<T>(&mut self, f: impl FnOnce(&mut MemStore) -> T) -> T {
        match self {
            HostStore::Memory(store) => f(store),
            _ => unreachable!(),
        }
    }
}

impl TxAccessor for HostStore {
    fn run_in_tx<T>(&self, tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T> {
        match self {
            #[cfg(feature = "rocksdbstore")]
            HostStore::RocksDB(store) => store.run_in_tx(tx_id, f),
            HostStore::Memory(store) => store.run_in_tx(tx_id, f),
        }
    }

    fn run_in_mut_tx<T>(
        &mut self,
        tx_id: TxId,
        f: impl FnOnce(&mut dyn KVStore) -> T,
    ) -> Result<T> {
        match self {
            #[cfg(feature = "rocksdbstore")]
            HostStore::RocksDB(store) => store.run_in_mut_tx(tx_id, f),
            HostStore::Memory(store) => store.run_in_mut_tx(tx_id, f),
        }
    }
}

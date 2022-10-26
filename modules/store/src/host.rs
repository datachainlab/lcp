use crate::memory::MemStore;
use crate::prelude::*;
use crate::transaction::{CommitStore, TxStore};
use crate::{KVStore, Result, TxId};

pub enum HostStore {
    #[cfg(feature = "rocksdbstore")]
    RocksDB(crate::rocksdb::RocksDBStore),
    Memory(crate::memory::MemStore),
}

pub trait HostStoreAccessor<S: CommitStore> {
    fn use_mut_store<T>(&self, f: impl FnOnce(&mut S) -> T) -> T;
}

pub trait HostCommitStore<S: CommitStore> {
    fn apply<T>(&mut self, f: impl FnOnce(&mut S) -> T) -> T;
}

#[cfg(feature = "rocksdbstore")]
impl HostCommitStore<crate::rocksdb::RocksDBStore> for HostStore {
    fn apply<T>(&mut self, f: impl FnOnce(&mut crate::rocksdb::RocksDBStore) -> T) -> T {
        match self {
            HostStore::RocksDB(store) => f(store),
            _ => unreachable!(),
        }
    }
}

impl HostCommitStore<MemStore> for HostStore {
    fn apply<T>(&mut self, f: impl FnOnce(&mut MemStore) -> T) -> T {
        match self {
            HostStore::Memory(store) => f(store),
            _ => unreachable!(),
        }
    }
}

impl TxStore for HostStore {
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

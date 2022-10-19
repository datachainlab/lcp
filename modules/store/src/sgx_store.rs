use crate::memory::MemStore;
use crate::{prelude::*, Error};
use crate::{KVStore, Store, TransactionStore};
use sgx_tstd::sync::{Arc, SgxRwLock};

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

impl<T> TransactionStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn begin(&mut self) -> Result<(), Error> {
        self.write().unwrap().begin()
    }

    fn commit(&mut self) -> Result<(), Error> {
        self.write().unwrap().commit()
    }

    fn abort(&mut self) {
        self.write().unwrap().abort()
    }
}

impl Store for Arc<SgxRwLock<MemStore>> {}

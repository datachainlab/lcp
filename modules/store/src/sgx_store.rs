use crate::memory::MemStore;
use crate::{prelude::*, Error};
use crate::{CommitStore, KVStore, Store};
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

impl<T> CommitStore for Arc<SgxRwLock<T>>
where
    T: Store,
{
    fn commit(&mut self) -> Result<(), Error> {
        self.write().unwrap().commit()
    }

    fn rollback(&mut self) {
        self.write().unwrap().rollback()
    }
}

impl Store for Arc<SgxRwLock<MemStore>> {}

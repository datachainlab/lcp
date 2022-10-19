use crate::prelude::*;
use crate::Error;
pub trait Store: KVStore + TransactionStore {}
pub trait KVStore {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>);
    fn get(&self, k: &[u8]) -> Option<Vec<u8>>;
}

pub trait TransactionStore {
    fn begin(&mut self) -> Result<(), Error>;
    fn commit(&mut self) -> Result<(), Error>;
    fn abort(&mut self);
}

impl KVStore for Box<dyn KVStore> {
    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        self.as_ref().get(k)
    }
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.as_mut().set(k, v)
    }
}

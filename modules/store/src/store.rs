use crate::prelude::*;
use crate::Error;

pub trait Store: KVStore + CommitStore {}

pub trait KVStore {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>);
    fn get(&self, k: &[u8]) -> Option<Vec<u8>>;
}

pub trait CommitStore {
    fn commit(&mut self) -> Result<(), Error>;
    fn rollback(&mut self);
}

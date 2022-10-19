use crate::prelude::*;
use store::KVStore;

#[derive(Default)]
pub struct EnclaveStore {}

impl KVStore for EnclaveStore {
    fn get(&self, k: &[u8]) -> Option<alloc::vec::Vec<u8>> {
        todo!()
    }
    fn set(&mut self, k: alloc::vec::Vec<u8>, v: alloc::vec::Vec<u8>) {
        todo!()
    }
}

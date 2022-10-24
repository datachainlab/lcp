use crate::prelude::*;
use store::{KVStore, TxId};

pub struct EnclaveStore {
    tx_id: TxId,
}

impl EnclaveStore {
    pub fn new(tx_id: TxId) -> Self {
        Self { tx_id }
    }
}

impl KVStore for EnclaveStore {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        host_api::store::get(self.tx_id, key.to_vec()).unwrap()
    }
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        host_api::store::set(self.tx_id, key, value).unwrap();
    }
    fn remove(&mut self, key: &[u8]) {
        host_api::store::remove(self.tx_id, key.to_vec()).unwrap();
    }
}

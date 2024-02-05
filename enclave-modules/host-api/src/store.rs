use crate::prelude::*;
use crate::{api::execute_command, Error};
use ocall_commands::{Command, CommandResult, StoreCommand, StoreResult};
use store::cache::CacheKVS;
use store::{KVStore, TxId};

/// The store guarantees that reads a value from the host store only once per key
pub fn new_enclave_store(tx_id: TxId) -> Box<dyn KVStore> {
    Box::new(CacheKVS::new(TxStore::new(tx_id)))
}

/// TxStore is a KVStore implementation that uses the ocall_commands to interact with the
/// host's store.
struct TxStore {
    tx_id: TxId,
}

impl TxStore {
    pub fn new(tx_id: TxId) -> Self {
        Self { tx_id }
    }
}

impl KVStore for TxStore {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        get(self.tx_id, key.to_vec()).unwrap()
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        set(self.tx_id, key.clone(), value.clone()).unwrap();
    }

    fn remove(&mut self, key: &[u8]) {
        remove(self.tx_id, key.to_vec()).unwrap();
    }
}

fn get(tx_id: TxId, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
    let cmd = Command::Store(StoreCommand::Get(tx_id, key));
    if let CommandResult::Store(StoreResult::Get(v)) = execute_command(cmd)? {
        Ok(v)
    } else {
        unreachable!()
    }
}

fn set(tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<(), Error> {
    let cmd = Command::Store(StoreCommand::Set(tx_id, key, value));
    if let CommandResult::Store(StoreResult::Set) = execute_command(cmd)? {
        Ok(())
    } else {
        unreachable!()
    }
}

fn remove(tx_id: TxId, key: Vec<u8>) -> Result<(), Error> {
    let cmd = Command::Store(StoreCommand::Remove(tx_id, key));
    if let CommandResult::Store(StoreResult::Remove) = execute_command(cmd)? {
        Ok(())
    } else {
        unreachable!()
    }
}

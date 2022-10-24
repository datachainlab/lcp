use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use store::TxId;

#[derive(Serialize, Deserialize, Debug)]
pub enum StoreCommand {
    Get(TxId, Vec<u8>),
    Set(TxId, Vec<u8>, Vec<u8>),
    Remove(TxId, Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StoreResult {
    Get(Option<Vec<u8>>),
    Set,
    Remove,
}

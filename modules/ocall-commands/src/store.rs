use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum StoreCommand {
    Get(Vec<u8>),
    Set(Vec<u8>, Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StoreResult {
    Get(Option<Vec<u8>>),
    Set,
}

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogCommand {
    pub msg: Vec<u8>,
}

impl LogCommand {
    pub fn new(msg: Vec<u8>) -> Self {
        Self { msg }
    }
}

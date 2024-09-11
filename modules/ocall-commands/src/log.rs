use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogCommand {
    pub msg: Vec<u8>,
}

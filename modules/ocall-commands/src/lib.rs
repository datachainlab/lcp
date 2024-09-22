#![no_std]
#![allow(clippy::large_enum_variant)]
extern crate alloc;
pub use crate::log::LogCommand;
pub use crate::store::{StoreCommand, StoreResult};
use serde::{Deserialize, Serialize};

mod log;
mod store;

#[derive(Serialize, Deserialize, Debug)]
pub struct OCallCommand {
    pub cmd: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Log(LogCommand),
    Store(StoreCommand),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum CommandResult {
    Log,
    Store(StoreResult),
    CommandError(alloc::string::String),
}

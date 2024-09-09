#![no_std]
#![allow(clippy::large_enum_variant)]
extern crate alloc;
pub use crate::store::{StoreCommand, StoreResult};
use serde::{Deserialize, Serialize};

mod store;

#[derive(Serialize, Deserialize, Debug)]
pub struct OCallCommand {
    pub cmd: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Store(StoreCommand),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResult {
    Store(StoreResult),
    CommandError(alloc::string::String),
}

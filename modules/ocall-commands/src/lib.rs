#![no_std]
#![allow(incomplete_features)]
#![allow(clippy::large_enum_variant)]
#![feature(generic_const_exprs)]
extern crate alloc;
pub use crate::store::{StoreCommand, StoreResult};

mod store;
mod transmuter;

use serde::{Deserialize, Serialize};

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

#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
extern crate alloc;
pub use remote_attestation::{
    GetIASSocketResult, GetQuoteInput, GetQuoteResult, GetReportAttestationStatusInput,
    GetReportAttestationStatusResult, InitQuoteResult, RemoteAttestationCommand,
    RemoteAttestationResult,
};
pub use store::{StoreCommand, StoreResult};

mod remote_attestation;
mod store;
mod transmuter;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OCallCommand {
    pub cmd: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    RemoteAttestation(RemoteAttestationCommand),
    Store(StoreCommand),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResult {
    RemoteAttestation(RemoteAttestationResult),
    Store(StoreResult),
    CommandError(alloc::string::String),
}

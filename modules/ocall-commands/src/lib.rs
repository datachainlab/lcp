#![no_std]
#![feature(generic_const_exprs)]
extern crate alloc;
pub use remote_attestation::{
    GetIASSocketResult, GetQuoteInput, GetQuoteResult, GetReportAttestationStatusInput,
    GetReportAttestationStatusResult, InitQuoteResult, RemoteAttestationCommand,
    RemoteAttestationResult,
};
use serde::{Deserialize, Serialize};

mod remote_attestation;
mod transmuter;

#[derive(Serialize, Deserialize, Debug)]
pub struct OCallCommand {
    pub cmd: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    RemoteAttestation(RemoteAttestationCommand),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResult {
    RemoteAttestation(RemoteAttestationResult),
    CommandError(alloc::string::String),
}

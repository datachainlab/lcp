#![allow(clippy::large_enum_variant)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod prelude {
    pub use core::prelude::v1::*;

    // Re-export according to alloc::prelude::v1 because it is not yet stabilized
    // https://doc.rust-lang.org/src/alloc/prelude/v1.rs.html
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;

    pub use alloc::format;
    pub use alloc::vec;

    // Those are exported by default in the std prelude in Rust 2021
    pub use core::convert::{TryFrom, TryInto};
    pub use core::iter::FromIterator;
}

pub use commands::{Command, CommandContext, CommandResponse, ECallCommand};
use crypto::Address;
pub use enclave_manage::{
    EnclaveManageCommand, EnclaveManageResponse, GenerateEnclaveKeyInput,
    GenerateEnclaveKeyResponse, IASRemoteAttestationInput, IASRemoteAttestationResponse,
};
#[cfg(feature = "sgx-sw")]
pub use enclave_manage::{SimulateRemoteAttestationInput, SimulateRemoteAttestationResponse};
pub use errors::InputValidationError;
pub use light_client::{
    AggregateMessagesInput, AggregateMessagesResponse, CommitmentProofPair, InitClientInput,
    InitClientResponse, LightClientCommand, LightClientExecuteCommand, LightClientQueryCommand,
    LightClientResponse, QueryClientInput, QueryClientResponse, UpdateClientInput,
    UpdateClientResponse, VerifyMembershipInput, VerifyMembershipResponse,
    VerifyNonMembershipInput, VerifyNonMembershipResponse,
};

mod commands;
mod enclave_manage;
mod errors;
mod light_client;
#[cfg(feature = "std")]
pub mod msgs;

pub trait EnclaveKeySelector {
    fn get_enclave_key(&self) -> Option<Address>;
}

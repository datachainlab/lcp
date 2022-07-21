#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}
// A trick to suppress an IDE error
#[cfg(not(feature = "sgx"))]
pub use anyhow;
#[cfg(feature = "sgx")]
pub use anyhow_sgx as anyhow;

use serde::{Deserialize, Serialize};
use std::string::String;

pub use enclave_manage::{
    EnclaveManageCommand, EnclaveManageResult, InitEnclaveInput, InitEnclaveResult,
};
pub use light_client::{
    CommitmentProofPair, InitClientInput, InitClientResult, LightClientCommand, LightClientResult,
    UpdateClientInput, UpdateClientResult, VerifyChannelInput, VerifyChannelResult,
    VerifyClientConsensusInput, VerifyClientConsensusResult, VerifyClientInput, VerifyClientResult,
    VerifyConnectionInput, VerifyConnectionResult,
};

mod enclave_manage;
mod light_client;

#[derive(Serialize, Deserialize, Debug)]
pub struct EnclaveCommand {
    pub params: CommandParams,
    pub cmd: Command,
}

impl EnclaveCommand {
    pub fn new(params: CommandParams, cmd: Command) -> Self {
        Self { params, cmd }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandParams {
    pub home: String,
}

impl CommandParams {
    pub fn new(home: String) -> Self {
        Self { home }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    EnclaveManage(EnclaveManageCommand),
    LightClient(LightClientCommand),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResult {
    EnclaveManage(EnclaveManageResult),
    LightClient(LightClientResult),
}

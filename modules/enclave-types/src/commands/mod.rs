use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::string::String;
use std::vec::Vec;

pub use enclave_manage::{
    EnclaveManageCommand, EnclaveManageResult, InitEnclaveInput, InitEnclaveResult,
};
pub use light_client::{
    CommitmentProof, InitClientInput, InitClientResult, LightClientCommand, LightClientResult,
    UpdateClientInput, UpdateClientResult, VerifyClientInput, VerifyClientResult,
};

mod enclave_manage;
mod light_client;

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

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(remote = "Any")]
pub struct AnyDef {
    pub type_url: String,
    pub value: Vec<u8>,
}

impl From<AnyDef> for Any {
    fn from(value: AnyDef) -> Self {
        Any {
            type_url: value.type_url,
            value: value.value,
        }
    }
}

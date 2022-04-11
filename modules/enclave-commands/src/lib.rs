#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use commitments_sgx as commitments;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}
// A trick to suppress an IDE error
#[cfg(not(feature = "sgx"))]
pub use anyhow;
#[cfg(feature = "sgx")]
pub use anyhow_sgx as anyhow;

use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::string::String;
use std::vec::Vec;

pub use enclave_manage::{
    EnclaveManageCommand, EnclaveManageResult, InitEnclaveInput, InitEnclaveResult,
};
pub use light_client::{
    InitClientInput, InitClientResult, LightClientCommand, LightClientResult, UpdateClientInput,
    UpdateClientResult, VerifyClientInput, VerifyClientResult,
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

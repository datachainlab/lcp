use crate::commands::AnyDef;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::ValidityProof;
use ibc::core::ics24_host::identifier::ClientId;
use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::string::String;

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientCommand {
    InitClient(InitClientInput),
    UpdateClient(UpdateClientInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientInput {
    pub client_type: String,
    #[serde(with = "AnyDef")]
    pub any_client_state: Any,
    #[serde(with = "AnyDef")]
    pub any_consensus_state: Any,
    pub current_timestamp: u64, // verification context
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientInput {
    pub client_id: ClientId,
    #[serde(with = "AnyDef")]
    pub any_header: Any,
    pub current_timestamp: u64, // verification context
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientResult {
    InitClient(InitClientResult),
    UpdateClient(UpdateClientResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientResult {
    pub proof: ValidityProof,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientResult {
    pub proof: ValidityProof,
}

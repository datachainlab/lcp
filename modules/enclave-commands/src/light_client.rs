#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::AnyDef;
use commitments::{StateCommitmentProof, UpdateClientCommitmentProof};
use ibc::core::{ics02_client::height::Height, ics24_host::identifier::ClientId};
use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::string::String;
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientCommand {
    InitClient(InitClientInput),
    UpdateClient(UpdateClientInput),
    VerifyClient(VerifyClientInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientInput {
    pub client_type: String,
    #[serde(with = "AnyDef")]
    pub any_client_state: Any,
    #[serde(with = "AnyDef")]
    pub any_consensus_state: Any,
    pub current_timestamp: u128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientInput {
    pub client_id: ClientId,
    #[serde(with = "AnyDef")]
    pub any_header: Any,
    pub current_timestamp: u128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientInput {
    pub client_id: ClientId,
    #[serde(with = "AnyDef")]
    pub target_any_client_state: Any,
    pub prefix: Vec<u8>,
    pub counterparty_client_id: ClientId,
    pub proof: TargetProof,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TargetProof(pub Height, pub Vec<u8>);

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientResult {
    InitClient(InitClientResult),
    UpdateClient(UpdateClientResult),
    VerifyClient(VerifyClientResult),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct InitClientResult(pub UpdateClientCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UpdateClientResult(pub UpdateClientCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientResult(pub StateCommitmentProof);

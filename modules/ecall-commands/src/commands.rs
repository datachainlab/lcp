use crate::{
    prelude::*, EnclaveKeySelector, EnclaveManageCommand, EnclaveManageResponse,
    LightClientCommand, LightClientResponse,
};
use crypto::SealedEnclaveKey;
use lcp_types::Time;
use serde::{Deserialize, Serialize};
use store::TxId;

#[derive(Serialize, Deserialize, Debug)]
pub struct ECallCommand {
    pub ctx: CommandContext,
    pub cmd: Command,
}

impl ECallCommand {
    pub fn new(ctx: CommandContext, cmd: Command) -> Self {
        Self { ctx, cmd }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandContext {
    pub current_timestamp: Time,
    pub sealed_ek: Option<SealedEnclaveKey>,
    pub tx_id: TxId,
}

impl CommandContext {
    pub fn new(current_timestamp: Time, sealed_ek: Option<SealedEnclaveKey>, tx_id: TxId) -> Self {
        Self {
            current_timestamp,
            sealed_ek,
            tx_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    EnclaveManage(EnclaveManageCommand),
    LightClient(LightClientCommand),
}

impl EnclaveKeySelector for Command {
    fn get_enclave_key(&self) -> Option<crypto::Address> {
        match self {
            Self::EnclaveManage(_) => None,
            Self::LightClient(cmd) => cmd.get_enclave_key(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResponse {
    EnclaveManage(EnclaveManageResponse),
    LightClient(LightClientResponse),
    CommandError(String),
}

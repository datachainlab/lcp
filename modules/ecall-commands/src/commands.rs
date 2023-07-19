use crate::{
    prelude::*, EnclaveKeySelector, EnclaveManageCommand, EnclaveManageResult, LightClientCommand,
    LightClientResult,
};
use crypto::SealedEnclaveKey;
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
    pub sealed_ek: Option<SealedEnclaveKey>,
    pub tx_id: TxId,
}

impl CommandContext {
    pub fn new(sealed_ek: Option<SealedEnclaveKey>, tx_id: TxId) -> Self {
        Self { sealed_ek, tx_id }
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
            Self::EnclaveManage(cmd) => cmd.get_enclave_key(),
            Self::LightClient(cmd) => cmd.get_enclave_key(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandResult {
    EnclaveManage(EnclaveManageResult),
    LightClient(LightClientResult),
    CommandError(String),
}

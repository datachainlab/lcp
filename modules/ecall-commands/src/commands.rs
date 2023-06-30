use crate::{
    prelude::*, EnclaveManageCommand, EnclaveManageResult, LightClientCommand, LightClientResult,
};
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
    pub sealed_ek: Vec<u8>,
    pub tx_id: TxId,
}

impl CommandContext {
    pub fn new(sealed_ek: Vec<u8>, tx_id: TxId) -> Self {
        Self { sealed_ek, tx_id }
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
    CommandError(String),
}

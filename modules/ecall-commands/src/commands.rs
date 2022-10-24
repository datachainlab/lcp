use crate::{
    prelude::*, EnclaveManageCommand, EnclaveManageResult, LightClientCommand, LightClientResult,
};
use serde::{Deserialize, Serialize};
use store::TxId;

#[derive(Serialize, Deserialize, Debug)]
pub struct ECallCommand {
    pub params: CommandParams,
    pub cmd: Command,
}

impl ECallCommand {
    pub fn new(params: CommandParams, cmd: Command) -> Self {
        Self { params, cmd }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandParams {
    pub home: String,
    pub tx_id: TxId,
}

impl CommandParams {
    pub fn new(home: String, tx_id: TxId) -> Self {
        Self { home, tx_id }
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

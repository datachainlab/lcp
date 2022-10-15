use crate::{
    prelude::*, EnclaveManageCommand, EnclaveManageResult, LightClientCommand, LightClientResult,
};
use serde::{Deserialize, Serialize};

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
    CommandError(String),
}

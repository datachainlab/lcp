use crate::enclave_manage;
use crate::light_client;
use crate::{Error, Result};
use ecall_commands::{Command, CommandResponse, ECallCommand};
use enclave_environment::Env;

pub fn dispatch<E: Env>(env: E, command: ECallCommand) -> Result<CommandResponse> {
    match command.cmd {
        Command::EnclaveManage(cmd) => {
            enclave_manage::dispatch(command.ctx, cmd).map_err(Error::enclave_manage_command)
        }
        Command::LightClient(cmd) => {
            light_client::dispatch(env, command.ctx, cmd).map_err(Error::light_client_command)
        }
    }
}

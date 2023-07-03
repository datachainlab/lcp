use crate::enclave_manage;
use crate::light_client;
use crate::prelude::*;
use crate::{Error, Result};
use context::Context;
use ecall_commands::{Command, CommandResult, ECallCommand};
use enclave_environment::Env;

pub fn dispatch<E: Env>(env: E, command: ECallCommand) -> Result<CommandResult> {
    match command.cmd {
        Command::EnclaveManage(cmd) => {
            enclave_manage::dispatch(command.ctx, cmd).map_err(Error::enclave_manage_command)
        }
        Command::LightClient(cmd) => {
            let signer = command
                .ctx
                .sealed_ek
                .ok_or(Error::sealed_enclave_key_not_found())?;
            let mut ctx = Context::new(
                env.get_lc_registry(),
                env.new_store(command.ctx.tx_id),
                &signer,
            );
            match light_client::dispatch(&mut ctx, cmd) {
                Ok(res) => Ok(res),
                Err(e) => {
                    return Err(Error::light_client_command(e));
                }
            }
        }
    }
}

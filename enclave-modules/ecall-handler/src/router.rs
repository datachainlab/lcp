use crate::enclave_manage;
use crate::light_client;
use crate::prelude::*;
use crate::{Error, Result};
use context::Context;
use crypto::EnclaveKey;
use ecall_commands::{Command, CommandResult, ECallCommand};
use enclave_environment::Environment;
use store::Store;

pub fn dispatch<'l, S: Store>(
    env: &Environment,
    ek: Option<&EnclaveKey>,
    store: &mut S,
    command: ECallCommand,
) -> Result<CommandResult> {
    let res = match command.cmd {
        Command::EnclaveManage(cmd) => {
            enclave_manage::dispatch(cmd, command.params).map_err(Error::enclave_manage_command)?
        }
        cmd => {
            let mut ctx = match ek {
                None => return Err(Error::enclave_key_not_found()),
                Some(ek) => Context::new(env, store, ek),
            };
            match cmd {
                Command::LightClient(cmd) => match light_client::dispatch(&mut ctx, cmd) {
                    Ok(res) => {
                        let _ = store.commit().map_err(Error::store)?;
                        res
                    }
                    Err(e) => {
                        store.rollback();
                        return Err(Error::light_client_command(e));
                    }
                },
                _ => unreachable!(),
            }
        }
    };
    Ok(res)
}

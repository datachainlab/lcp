use crate::{errors::Result, remote_attestation, store};
use host_environment::Environment;
use ocall_commands::{Command, CommandResult, OCallCommand};

pub fn dispatch(env: &Environment, command: OCallCommand) -> Result<CommandResult> {
    match command.cmd {
        Command::RemoteAttestation(cmd) => Ok(CommandResult::RemoteAttestation(
            remote_attestation::dispatch(cmd)?,
        )),
        Command::Store(cmd) => Ok(CommandResult::Store(store::dispatch(env, cmd)?)),
    }
}

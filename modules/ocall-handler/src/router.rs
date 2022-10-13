use crate::{errors::Result, remote_attestation};
use host_environment::Environment;
use ocall_commands::{Command, CommandResult, OCallCommand};

pub fn dispatch(_env: &Environment, command: OCallCommand) -> Result<CommandResult> {
    match command.cmd {
        Command::RemoteAttestation(cmd) => Ok(CommandResult::RemoteAttestation(
            remote_attestation::dispatch(cmd)?,
        )),
    }
}

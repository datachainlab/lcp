use crate::{errors::Result, remote_attestation};
use ocall_commands::{Command, CommandResult, OCallCommand};

pub fn dispatch(command: OCallCommand) -> Result<CommandResult> {
    match command.cmd {
        Command::RemoteAttestation(cmd) => Ok(CommandResult::RemoteAttestation(
            remote_attestation::dispatch(cmd)?,
        )),
    }
}

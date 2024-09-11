use crate::{errors::Result, store};
use host_environment::Environment;
use ocall_commands::{Command, CommandResult, OCallCommand};

pub fn dispatch(env: &Environment, command: OCallCommand) -> Result<CommandResult> {
    match command.cmd {
        Command::Log(cmd) => {
            crate::log::dispatch(cmd)?;
            Ok(CommandResult::Log)
        }
        Command::Store(cmd) => Ok(CommandResult::Store(store::dispatch(env, cmd)?)),
    }
}

use crate::errors::Error;
use ocall_commands::LogCommand;
use std::io::{stdout, Write};

pub fn dispatch(command: LogCommand) -> Result<(), Error> {
    stdout().write_all(&command.msg)?;
    stdout().flush()?;
    Ok(())
}

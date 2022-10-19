use crate::prelude::*;
use crate::{api::execute_command, Error};
use ocall_commands::{Command, CommandResult, StoreCommand, StoreResult};

pub fn get(k: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
    let cmd = Command::Store(StoreCommand::Get(k));
    if let CommandResult::Store(StoreResult::Get(v)) = execute_command(cmd)? {
        Ok(v)
    } else {
        unreachable!()
    }
}

pub fn set(k: Vec<u8>, v: Vec<u8>) -> Result<(), Error> {
    let cmd = Command::Store(StoreCommand::Set(k, v));
    if let CommandResult::Store(StoreResult::Set) = execute_command(cmd)? {
        Ok(())
    } else {
        unreachable!()
    }
}

use crate::prelude::*;
use crate::{api::execute_command, Error};
use ocall_commands::{Command, CommandResult, StoreCommand, StoreResult};
use store::TxId;

pub fn get(tx_id: TxId, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
    let cmd = Command::Store(StoreCommand::Get(tx_id, key));
    if let CommandResult::Store(StoreResult::Get(v)) = execute_command(cmd)? {
        Ok(v)
    } else {
        unreachable!()
    }
}

pub fn set(tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<(), Error> {
    let cmd = Command::Store(StoreCommand::Set(tx_id, key, value));
    if let CommandResult::Store(StoreResult::Set) = execute_command(cmd)? {
        Ok(())
    } else {
        unreachable!()
    }
}

pub fn remove(tx_id: TxId, key: Vec<u8>) -> Result<(), Error> {
    let cmd = Command::Store(StoreCommand::Remove(tx_id, key));
    if let CommandResult::Store(StoreResult::Remove) = execute_command(cmd)? {
        Ok(())
    } else {
        unreachable!()
    }
}

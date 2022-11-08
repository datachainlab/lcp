use crate::errors::Result;
use host_environment::Environment;
use ocall_commands::{StoreCommand, StoreResult};
use store::transaction::TxAccessor;

pub fn dispatch(env: &Environment, command: StoreCommand) -> Result<StoreResult> {
    let res = match command {
        StoreCommand::Get(tx_id, key) => StoreResult::Get(env.get_store().tx_get(tx_id, &key)?),
        StoreCommand::Set(tx_id, key, value) => {
            env.get_mut_store().tx_set(tx_id, key, value)?;
            StoreResult::Set
        }
        StoreCommand::Remove(tx_id, key) => {
            env.get_mut_store().tx_remove(tx_id, &key)?;
            StoreResult::Remove
        }
    };
    Ok(res)
}

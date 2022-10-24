use crate::errors::Result;
use host_environment::Environment;
use ocall_commands::{StoreCommand, StoreResult};

pub fn dispatch(env: &Environment, command: StoreCommand) -> Result<StoreResult> {
    use StoreCommand::*;
    // TODO add error handling
    let res = match command {
        Get(tx_id, key) => {
            StoreResult::Get(env.store.try_read().unwrap().tx_get(tx_id, &key).unwrap())
        }
        Set(tx_id, key, value) => {
            env.store
                .try_write()
                .unwrap()
                .tx_set(tx_id, key, value)
                .unwrap();
            StoreResult::Set
        }
        Remove(tx_id, key) => {
            env.store
                .try_write()
                .unwrap()
                .tx_remove(tx_id, &key)
                .unwrap();
            StoreResult::Remove
        }
    };
    Ok(res)
}

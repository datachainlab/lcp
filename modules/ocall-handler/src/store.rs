use crate::errors::Result;
use host_environment::Environment;
use ocall_commands::{StoreCommand, StoreResult};

pub fn dispatch(env: &Environment, command: StoreCommand) -> Result<StoreResult> {
    use StoreCommand::*;
    // TODO add error handling
    let res = match command {
        Get(k) => StoreResult::Get(env.store.try_read().unwrap().get(&k)),
        Set(k, v) => {
            env.store.try_write().unwrap().set(k, v);
            StoreResult::Set
        }
    };
    Ok(res)
}

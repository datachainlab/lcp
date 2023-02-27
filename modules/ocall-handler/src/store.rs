use crate::errors::Result;
use host_environment::Environment;
use log::*;
use ocall_commands::{StoreCommand, StoreResult};
use store::transaction::TxAccessor;

pub fn dispatch(env: &Environment, command: StoreCommand) -> Result<StoreResult> {
    let res = match command {
        StoreCommand::Get(tx_id, key) => {
            debug!(
                "Get: tx_id={} key={:?} key(utf8)={}",
                tx_id,
                key,
                String::from_utf8_lossy(&key)
            );
            StoreResult::Get(env.get_store().tx_get(tx_id, &key)?)
        }
        StoreCommand::Set(tx_id, key, value) => {
            debug!(
                "Set: tx_id={} key={:?} key(utf8)={} value={:?}",
                tx_id,
                key,
                String::from_utf8_lossy(&key),
                value
            );
            env.get_mut_store().tx_set(tx_id, key, value)?;
            StoreResult::Set
        }
        StoreCommand::Remove(tx_id, key) => {
            debug!(
                "Remove: tx_id={} key={:?} key(utf8)={}",
                tx_id,
                key,
                String::from_utf8_lossy(&key)
            );
            env.get_mut_store().tx_remove(tx_id, &key)?;
            StoreResult::Remove
        }
    };
    Ok(res)
}

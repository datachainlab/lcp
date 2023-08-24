use crate::prelude::*;
use alloc::sync::Arc;
use light_client::LightClientResolver;
use store::{KVStore, TxId};

pub trait Env: Sync + Send {
    fn new_store(&self, tx_id: TxId) -> Box<dyn KVStore>;

    fn get_lc_registry(&self) -> Arc<dyn LightClientResolver>;
}

impl Env for &Box<dyn Env> {
    fn new_store(&self, tx_id: TxId) -> Box<dyn KVStore> {
        self.as_ref().new_store(tx_id)
    }

    fn get_lc_registry(&self) -> Arc<dyn LightClientResolver> {
        self.as_ref().get_lc_registry()
    }
}

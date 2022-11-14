use crate::{prelude::*, Env};
use alloc::sync::Arc;
use enclave_store::EnclaveStore;
use light_client::LightClient;
use light_client_registry::{memory::HashMapLightClientRegistry, LightClientResolver};
use store::{KVStore, TxId};

pub struct Environment {
    lc_registry: Arc<HashMapLightClientRegistry>,
}

impl Environment {
    pub fn new(lc_registry: HashMapLightClientRegistry) -> Self {
        Self {
            lc_registry: Arc::new(lc_registry),
        }
    }
}

impl LightClientResolver for Environment {
    fn get_light_client(&self, type_url: &str) -> Option<&alloc::boxed::Box<dyn LightClient>> {
        self.lc_registry.get_light_client(type_url)
    }
}

impl Env for Environment {
    fn new_store(&self, tx_id: TxId) -> Box<dyn KVStore> {
        Box::new(EnclaveStore::new(tx_id))
    }

    fn get_lc_registry(&self) -> Arc<dyn LightClientResolver> {
        self.lc_registry.clone()
    }
}

unsafe impl Sync for Environment {}
unsafe impl Send for Environment {}

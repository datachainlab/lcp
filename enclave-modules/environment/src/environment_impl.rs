use crate::{prelude::*, Env};
use alloc::sync::Arc;
use host_api::store::new_enclave_store;
use light_client::{LightClient, LightClientResolver, MapLightClientRegistry};
use store::{KVStore, TxId};

pub struct Environment {
    lc_registry: Arc<MapLightClientRegistry>,
}

impl Environment {
    pub fn new(lc_registry: MapLightClientRegistry) -> Self {
        if !lc_registry.is_sealed() {
            panic!("Light client registry must be sealed before creating an environment");
        }
        Self {
            // Thread-safe: `lc_registry` is sealed and immutable after initialization
            #[allow(clippy::arc_with_non_send_sync)]
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
        new_enclave_store(tx_id)
    }

    fn get_lc_registry(&self) -> Arc<dyn LightClientResolver> {
        self.lc_registry.clone()
    }
}

unsafe impl Sync for Environment {}
unsafe impl Send for Environment {}

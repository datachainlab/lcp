use crate::{prelude::*, Env};
use alloc::rc::Rc;
use enclave_store::EnclaveStore;
use light_client::LightClient;
use light_client_registry::{memory::HashMapLightClientRegistry, LightClientResolver};
use store::KVStore;

pub struct Environment {
    lc_registry: Rc<HashMapLightClientRegistry>,
}

impl Environment {
    pub fn new(lc_registry: HashMapLightClientRegistry) -> Self {
        Self {
            lc_registry: Rc::new(lc_registry),
        }
    }
}

impl LightClientResolver for Environment {
    fn get_light_client(&self, type_url: &str) -> Option<&alloc::boxed::Box<dyn LightClient>> {
        self.lc_registry.get_light_client(type_url)
    }
}

impl Env for Environment {
    fn get_store(&self) -> Box<dyn KVStore> {
        Box::new(EnclaveStore {})
    }
    fn get_lc_registry(&self) -> Rc<dyn LightClientResolver> {
        self.lc_registry.clone()
    }
}

unsafe impl Sync for Environment {}
unsafe impl Send for Environment {}

use crate::prelude::*;
use light_client::LightClient;
use light_client_registry::{LightClientRegistry, LightClientResolver};

pub struct Environment {
    lc_registry: Box<dyn LightClientRegistry + Sync + Send>,
}

impl Environment {
    pub fn new(lc_registry: Box<dyn LightClientRegistry + Sync + Send>) -> Self {
        Self { lc_registry }
    }
}

impl LightClientResolver for Environment {
    fn get_light_client(&self, type_url: &str) -> Option<&alloc::boxed::Box<dyn LightClient>> {
        self.lc_registry.get(type_url)
    }
}

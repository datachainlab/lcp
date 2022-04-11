use enclave_light_client::{LightClient, LightClientRegistry, LightClientSource};
use enclave_tendermint::register_implementations;
use lazy_static::lazy_static;
use std::boxed::Box;

lazy_static! {
    pub static ref LIGHT_CLIENT_REGISTRY: LightClientRegistry = {
        let mut registry = LightClientRegistry::new();
        register_implementations(&mut registry);
        registry
    };
}

#[derive(Default)]
pub struct GlobalLightClientRegistry {}

impl LightClientSource<'_> for GlobalLightClientRegistry {
    fn get_light_client(client_type: &str) -> Option<&'static Box<dyn LightClient>> {
        LIGHT_CLIENT_REGISTRY.get(client_type)
    }
}

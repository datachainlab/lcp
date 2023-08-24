use crate::errors::RegistryError;
use crate::prelude::*;
use crate::LightClient;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

pub trait LightClientRegistry: LightClientResolver {
    fn put_light_client(
        &mut self,
        client_state_type_url: String,
        lc: Box<dyn LightClient>,
    ) -> Result<(), RegistryError>;
}

pub trait LightClientResolver {
    #[allow(clippy::borrowed_box)]
    fn get_light_client(&self, type_url: &str) -> Option<&Box<dyn LightClient>>;
}

impl LightClientResolver for Arc<dyn LightClientResolver> {
    fn get_light_client(&self, type_url: &str) -> Option<&Box<dyn LightClient>> {
        self.as_ref().get_light_client(type_url)
    }
}

#[derive(Default)]
pub struct MapLightClientRegistry {
    registry: BTreeMap<String, Box<dyn LightClient>>,
    sealed: bool,
}

impl MapLightClientRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn seal(&mut self) -> Result<(), RegistryError> {
        match self.sealed {
            true => Err(RegistryError::already_sealed()),
            false => {
                self.sealed = true;
                Ok(())
            }
        }
    }
}

impl LightClientRegistry for MapLightClientRegistry {
    fn put_light_client(
        &mut self,
        client_state_type_url: String,
        lc: Box<dyn LightClient>,
    ) -> Result<(), RegistryError> {
        assert!(!self.sealed);
        if self.get_light_client(&client_state_type_url).is_some() {
            Err(RegistryError::type_url_already_exists(
                client_state_type_url,
            ))
        } else {
            self.registry.insert(client_state_type_url, lc);
            Ok(())
        }
    }
}

impl LightClientResolver for MapLightClientRegistry {
    fn get_light_client(&self, client_state_type_url: &str) -> Option<&Box<dyn LightClient>> {
        self.registry.get(client_state_type_url)
    }
}

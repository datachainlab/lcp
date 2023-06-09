use crate::errors::Error;
use crate::registry::LightClientRegistry;
use crate::{prelude::*, LightClientResolver};
use light_client::LightClient;
use std::collections::HashMap;

#[derive(Default)]
pub struct HashMapLightClientRegistry {
    registry: HashMap<String, Box<dyn LightClient>>,
    sealed: bool,
}

impl HashMapLightClientRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn seal(&mut self) -> Result<(), Error> {
        match self.sealed {
            true => Err(Error::already_sealed()),
            false => {
                self.sealed = true;
                Ok(())
            }
        }
    }
}

impl LightClientRegistry for HashMapLightClientRegistry {
    fn put_light_client(
        &mut self,
        client_state_type_url: String,
        lc: Box<dyn LightClient>,
    ) -> Result<(), Error> {
        assert!(!self.sealed);
        if self.get_light_client(&client_state_type_url).is_some() {
            Err(Error::type_url_already_exists(client_state_type_url))
        } else {
            self.registry.insert(client_state_type_url, lc);
            Ok(())
        }
    }
}

impl LightClientResolver for HashMapLightClientRegistry {
    fn get_light_client(&self, client_state_type_url: &str) -> Option<&Box<dyn LightClient>> {
        self.registry.get(client_state_type_url)
    }
}

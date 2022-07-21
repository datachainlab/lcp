use crate::{
    errors::{LightClientError as Error, Result},
    LightClient,
};
use std::boxed::Box;
use std::collections::HashMap;
use std::string::String;

#[derive(Default)]
pub struct LightClientRegistry {
    registry: HashMap<String, Box<dyn LightClient>>,
    sealed: bool,
}

unsafe impl Send for LightClientRegistry {}
unsafe impl Sync for LightClientRegistry {}

impl LightClientRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn seal(&mut self) -> Result<()> {
        match self.sealed {
            true => Err(Error::AlreadySealedError().into()),
            false => {
                self.sealed = true;
                Ok(())
            }
        }
    }

    pub fn put(&mut self, client_state_type_url: String, lc: Box<dyn LightClient>) -> Result<()> {
        assert!(!self.sealed);
        if self.get(&client_state_type_url).is_some() {
            Err(Error::TypeUrlAlreadyExistsError(client_state_type_url))
        } else {
            self.registry.insert(client_state_type_url, lc);
            Ok(())
        }
    }

    pub fn get(&self, client_state_type_url: &str) -> Option<&Box<dyn LightClient>> {
        self.registry.get(client_state_type_url)
    }
}

pub trait LightClientSource<'a> {
    fn get_light_client(type_url: &str) -> Option<&'a Box<dyn LightClient>>;
}

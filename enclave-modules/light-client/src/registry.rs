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

    pub fn put(&mut self, client_type: String, lc: Box<dyn LightClient>) -> Result<()> {
        // TODO check if same type_url doesn't exist in the registry
        assert!(!self.sealed);
        self.registry.insert(client_type, lc);
        Ok(())
    }

    pub fn get(&self, client_type: &str) -> Option<&Box<dyn LightClient>> {
        match self.registry.get(client_type) {
            Some(lc) => Some(lc),
            None => None,
        }
    }
}

pub trait LightClientSource<'a> {
    fn get_light_client(type_url: &str) -> Option<&'a Box<dyn LightClient>>;
}

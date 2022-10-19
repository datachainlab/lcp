use crate::errors::Error;
use crate::prelude::*;
use alloc::rc::Rc;
use light_client::LightClient;

pub trait LightClientRegistry: LightClientResolver {
    fn put_light_client(
        &mut self,
        client_state_type_url: String,
        lc: Box<dyn LightClient>,
    ) -> Result<(), Error>;
}

pub trait LightClientResolver {
    fn get_light_client(&self, type_url: &str) -> Option<&Box<dyn LightClient>>;
}

impl LightClientResolver for Rc<dyn LightClientResolver> {
    fn get_light_client(&self, type_url: &str) -> Option<&Box<dyn LightClient>> {
        self.as_ref().get_light_client(type_url)
    }
}

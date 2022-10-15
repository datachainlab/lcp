use crate::errors::Error;
use crate::prelude::*;
use light_client::LightClient;

pub trait LightClientRegistry {
    fn put(&mut self, client_state_type_url: String, lc: Box<dyn LightClient>)
        -> Result<(), Error>;

    fn get(&self, client_state_type_url: &str) -> Option<&Box<dyn LightClient>>;
}

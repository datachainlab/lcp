use crate::prelude::*;
use alloc::rc::Rc;
use light_client_registry::LightClientResolver;
use store::KVStore;

pub trait Env: Sync + Send {
    fn get_store(&self) -> Box<dyn KVStore>;
    fn get_lc_registry(&self) -> Rc<dyn LightClientResolver>;
}

impl Env for &Box<dyn Env> {
    fn get_store(&self) -> Box<dyn KVStore> {
        self.as_ref().get_store()
    }
    fn get_lc_registry(&self) -> Rc<dyn LightClientResolver> {
        self.as_ref().get_lc_registry()
    }
}

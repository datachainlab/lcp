use crate::prelude::*;
use crypto::Signer;
use lcp_types::Time;
use light_client::{ClientKeeper, ClientReader, HostClientKeeper, HostClientReader, HostContext};
use light_client_registry::LightClientResolver;
use store::KVStore;

pub struct Context<'k, R: LightClientResolver, S: KVStore> {
    lc_registry: R,
    store: S,
    ek: &'k dyn Signer,
    current_timestamp: Option<Time>,
}

impl<'k, R: LightClientResolver, S: KVStore> Context<'k, R, S> {
    pub fn new(lc_registry: R, store: S, ek: &'k dyn Signer) -> Self {
        Self {
            lc_registry,
            store,
            ek,
            current_timestamp: None,
        }
    }

    pub fn set_timestamp(&mut self, timestamp: Time) {
        self.current_timestamp = Some(timestamp)
    }

    pub fn get_enclave_key(&self) -> &'k dyn Signer {
        self.ek
    }
}

impl<'k, R: LightClientResolver, S: KVStore> KVStore for Context<'k, R, S> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.store.set(key, value)
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.store.get(key)
    }

    fn remove(&mut self, key: &[u8]) {
        self.store.remove(key)
    }
}

impl<'k, R: LightClientResolver, S: KVStore> HostContext for Context<'k, R, S> {
    fn host_timestamp(&self) -> Time {
        self.current_timestamp.unwrap()
    }
}

impl<'k, R: LightClientResolver, S: KVStore> ClientReader for Context<'k, R, S> {}

impl<'k, R: LightClientResolver, S: KVStore> ClientKeeper for Context<'k, R, S> {}

impl<'k, R: LightClientResolver, S: KVStore> HostClientReader for Context<'k, R, S> {}

impl<'k, R: LightClientResolver, S: KVStore> HostClientKeeper for Context<'k, R, S> {}

impl<'k, R: LightClientResolver, S: KVStore> LightClientResolver for Context<'k, R, S> {
    fn get_light_client(
        &self,
        type_url: &str,
    ) -> Option<&alloc::boxed::Box<dyn light_client::LightClient>> {
        self.lc_registry.get_light_client(type_url)
    }
}

use crate::prelude::*;
use crypto::Signer;
use ibc::{
    core::{
        ics02_client::error::ClientError as ICS02Error,
        ics24_host::{
            identifier::ClientId,
            path::{ClientConsensusStatePath, ClientStatePath, ClientTypePath},
        },
    },
    timestamp::Timestamp,
};
use lcp_types::{Any, Height, Time};
use light_client::{ClientKeeper, ClientReader};
use light_client_registry::LightClientResolver;
use log::*;
use store::KVStore;

pub static NEXT_CLIENT_SEQUENCE: &str = "nextClientSequence";

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

impl<'k, R: LightClientResolver, S: KVStore> ClientReader for Context<'k, R, S> {
    fn client_type(&self, client_id: &ClientId) -> Result<String, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientTypePath(client_id.clone())).as_bytes());
        if value.is_none() {
            return Err(ICS02Error::ClientNotFound {
                client_id: client_id.clone(),
            });
        }
        Ok(String::from_utf8(value.unwrap()).unwrap())
    }

    fn client_state(&self, client_id: &ClientId) -> Result<Any, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientStatePath(client_id.clone())).as_bytes());
        if value.is_none() {
            return Err(ICS02Error::ClientNotFound {
                client_id: client_id.clone(),
            });
        }
        Ok(bincode::deserialize(&value.unwrap()).unwrap())
    }

    fn consensus_state(&self, client_id: &ClientId, height: Height) -> Result<Any, ICS02Error> {
        let path = ClientConsensusStatePath {
            client_id: client_id.clone(),
            epoch: height.revision_number(),
            height: height.revision_height(),
        };
        debug!("consensus_state: height={:?}", height);
        let value = match self.store.get(format!("{}", path).as_bytes()) {
            Some(value) => value,
            None => {
                return Err(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone(),
                    height: height.try_into()?,
                });
            }
        };
        Ok(bincode::deserialize(&value).unwrap())
    }

    fn host_height(&self) -> Height {
        // always return zero
        Default::default()
    }

    fn host_timestamp(&self) -> Timestamp {
        self.current_timestamp.unwrap().into()
    }

    fn client_counter(&self) -> Result<u64, ICS02Error> {
        match self.store.get(NEXT_CLIENT_SEQUENCE.as_bytes()) {
            Some(bz) => {
                let mut b: [u8; 8] = Default::default();
                b.copy_from_slice(&bz);
                Ok(u64::from_be_bytes(b))
            }
            None => Ok(0),
        }
    }
}

impl<'k, R: LightClientResolver, S: KVStore> ClientKeeper for Context<'k, R, S> {
    fn store_client_type(
        &mut self,
        client_id: ClientId,
        client_type: String,
    ) -> Result<(), ICS02Error> {
        self.store.set(
            format!("{}", ClientTypePath(client_id)).into_bytes(),
            client_type.into_bytes(),
        );
        Ok(())
    }

    fn store_any_client_state(
        &mut self,
        client_id: ClientId,
        client_state: Any,
    ) -> Result<(), ICS02Error> {
        let bz = bincode::serialize(&client_state).unwrap();
        self.store
            .set(format!("{}", ClientStatePath(client_id)).into_bytes(), bz);
        Ok(())
    }

    fn store_any_consensus_state(
        &mut self,
        client_id: ClientId,
        height: Height,
        consensus_state: Any,
    ) -> Result<(), ICS02Error> {
        let bz = bincode::serialize(&consensus_state).unwrap();
        let path = ClientConsensusStatePath {
            client_id,
            epoch: height.revision_number(),
            height: height.revision_height(),
        };
        self.store.set(format!("{}", path).into_bytes(), bz);
        Ok(())
    }

    fn increase_client_counter(&mut self) {
        let next_counter = <Self as ClientReader>::client_counter(self).unwrap() + 1;
        self.store.set(
            NEXT_CLIENT_SEQUENCE.as_bytes().to_vec(),
            next_counter.to_be_bytes().to_vec(),
        );
    }

    fn store_update_time(
        &mut self,
        _client_id: ClientId,
        _height: Height,
        _timestamp: Timestamp,
    ) -> Result<(), ICS02Error> {
        Ok(())
    }

    fn store_update_height(
        &mut self,
        _client_id: ClientId,
        _height: Height,
        _host_height: Height,
    ) -> Result<(), ICS02Error> {
        Ok(())
    }
}

impl<'k, R: LightClientResolver, S: KVStore> LightClientResolver for Context<'k, R, S> {
    fn get_light_client(
        &self,
        type_url: &str,
    ) -> Option<&alloc::boxed::Box<dyn light_client::LightClient>> {
        self.lc_registry.get_light_client(type_url)
    }
}

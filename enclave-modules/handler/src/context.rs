use context::{LightClientKeeper, LightClientReader};
use core::str::FromStr;
use enclave_crypto::EnclaveKey;
use enclave_store::Store;
use ibc::{
    core::{
        ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState,
            client_type::ClientType, context::ClientReader, error::Error as ICS02Error,
        },
        ics03_connection::context::ConnectionReader,
        ics03_connection::error::Error as ICS03Error,
        ics23_commitment::commitment::CommitmentPrefix,
        ics24_host::{
            identifier::{ClientId, ConnectionId},
            path::{ClientConsensusStatePath, ClientStatePath, ClientTypePath},
        },
    },
    timestamp::Timestamp,
    Height,
};
use log::*;
use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::format;
use std::string::String;
use std::vec::Vec;

pub struct Context<'a, 'e, S> {
    store: &'a mut S,
    ek: &'e EnclaveKey,
    current_timestamp: Option<u128>,
}

impl<'a, 'e, S> Context<'a, 'e, S> {
    pub fn new(store: &'a mut S, ek: &'e EnclaveKey) -> Self {
        Self {
            store,
            ek,
            current_timestamp: None,
        }
    }

    pub fn set_timestmap(&mut self, timestamp: u128) {
        self.current_timestamp = Some(timestamp);
    }

    pub fn get_enclave_key(&self) -> &'e EnclaveKey {
        self.ek
    }
}

impl<'a, 'e, S: Store> LightClientReader for Context<'a, 'e, S> {
    fn client_type(&self, client_id: &ClientId) -> Result<String, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientTypePath(client_id.clone())).as_bytes());
        Ok(String::from_utf8(value.unwrap()).unwrap())
    }

    fn client_state(&self, client_id: &ClientId) -> Result<Any, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientStatePath(client_id.clone())).as_bytes());
        let any: SerializableAny = bincode::deserialize(&value.unwrap()).unwrap();
        Ok(any.into())
    }

    fn consensus_state(&self, client_id: &ClientId, height: Height) -> Result<Any, ICS02Error> {
        let path = ClientConsensusStatePath {
            client_id: client_id.clone(),
            epoch: height.revision_number,
            height: height.revision_height,
        };
        debug!("consensus_state: height={:?}", height);
        let value = match self.store.get(format!("{}", path).as_bytes()) {
            Some(value) => value,
            None => {
                // TODO fix
                return Err(ICS02Error::consensus_state_not_found(
                    client_id.clone(),
                    height,
                ));
                // panic!("{:?}, {:?}", client_id, height);
            }
        };
        let any: SerializableAny = bincode::deserialize(&value).unwrap();
        Ok(any.into())
    }

    fn host_height(&self) -> Height {
        // always return zero
        Default::default()
    }

    fn host_timestamp(&self) -> Timestamp {
        Timestamp::from_nanoseconds(self.current_timestamp.unwrap() as u64).unwrap()
    }
}

impl<'a, 'e, S: Store> ClientReader for Context<'a, 'e, S> {
    fn client_type(&self, client_id: &ClientId) -> Result<ClientType, ICS02Error> {
        let client_type = <Self as LightClientReader>::client_type(&self, client_id)?;
        ClientType::from_str(&client_type)
    }

    fn client_state(&self, client_id: &ClientId) -> Result<AnyClientState, ICS02Error> {
        let client_state = <Self as LightClientReader>::client_state(&self, client_id)?;
        AnyClientState::try_from(client_state)
    }

    fn consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<AnyConsensusState, ICS02Error> {
        let consensus_state =
            <Self as LightClientReader>::consensus_state(&self, client_id, height)?;
        AnyConsensusState::try_from(consensus_state)
    }

    /// Similar to `consensus_state`, attempt to retrieve the consensus state,
    /// but return `None` if no state exists at the given height.
    fn maybe_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        use ibc::core::ics02_client::error::ErrorDetail;
        debug!("maybe_consensus_state: {:?}", height);
        match <Self as ClientReader>::consensus_state(&self, client_id, height) {
            Ok(cs) => Ok(Some(cs)),
            Err(e) => match e.detail() {
                ErrorDetail::ConsensusStateNotFound(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    fn next_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        todo!()
    }

    fn prev_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        // TODO implement this
        Ok(None)
    }

    fn host_height(&self) -> Height {
        <Self as LightClientReader>::host_height(&self)
    }

    fn host_timestamp(&self) -> Timestamp {
        <Self as LightClientReader>::host_timestamp(&self)
    }

    fn host_consensus_state(&self, height: Height) -> Result<AnyConsensusState, ICS02Error> {
        todo!()
    }

    fn pending_host_consensus_state(&self) -> Result<AnyConsensusState, ICS02Error> {
        todo!()
    }

    fn client_counter(&self) -> Result<u64, ICS02Error> {
        todo!()
    }
}

impl<'a, 'e, S: Store> ConnectionReader for Context<'a, 'e, S> {
    fn connection_end(
        &self,
        conn_id: &ConnectionId,
    ) -> Result<ibc::core::ics03_connection::connection::ConnectionEnd, ICS03Error> {
        todo!()
    }

    fn client_state(&self, client_id: &ClientId) -> Result<AnyClientState, ICS03Error> {
        let client_state = <Self as ClientReader>::client_state(&self, client_id)
            .map_err(ICS03Error::ics02_client)?;
        Ok(client_state)
    }

    fn host_current_height(&self) -> Height {
        todo!()
    }

    fn host_oldest_height(&self) -> Height {
        todo!()
    }

    fn commitment_prefix(&self) -> CommitmentPrefix {
        todo!()
    }

    fn client_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<AnyConsensusState, ICS03Error> {
        let consensus_state = <Self as ClientReader>::consensus_state(&self, client_id, height)
            .map_err(ICS03Error::ics02_client)?;
        Ok(consensus_state)
    }

    fn host_consensus_state(&self, height: Height) -> Result<AnyConsensusState, ICS03Error> {
        todo!()
    }

    fn connection_counter(&self) -> Result<u64, ICS03Error> {
        todo!()
    }
}

impl<'a, 'e, S: Store> LightClientKeeper for Context<'a, 'e, S> {
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
        let any: SerializableAny = client_state.into();
        let bz = bincode::serialize(&any).unwrap();
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
        let any: SerializableAny = consensus_state.into();
        let bz = bincode::serialize(&any).unwrap();
        let path = ClientConsensusStatePath {
            client_id,
            epoch: height.revision_number,
            height: height.revision_height,
        };
        self.store.set(format!("{}", path).into_bytes(), bz);
        Ok(())
    }

    fn increase_client_counter(&mut self) {}

    fn store_update_time(
        &mut self,
        client_id: ClientId,
        height: Height,
        timestamp: Timestamp,
    ) -> Result<(), ICS02Error> {
        Ok(())
    }

    fn store_update_height(
        &mut self,
        client_id: ClientId,
        height: Height,
        host_height: Height,
    ) -> Result<(), ICS02Error> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableAny {
    pub type_url: String,
    pub value: Vec<u8>,
}

impl Into<SerializableAny> for Any {
    fn into(self) -> SerializableAny {
        SerializableAny {
            type_url: self.type_url,
            value: self.value,
        }
    }
}

impl From<SerializableAny> for Any {
    fn from(value: SerializableAny) -> Self {
        Any {
            type_url: value.type_url,
            value: value.value,
        }
    }
}

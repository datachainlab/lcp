use crate::prelude::*;
use core::str::FromStr;
use crypto::Signer;
use ibc::{
    core::{
        ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState,
            client_type::ClientType, context::ClientReader as IBCClientReader,
            error::Error as ICS02Error, height::Height as ICS02Height,
        },
        ics24_host::{
            identifier::ClientId,
            path::{ClientConsensusStatePath, ClientStatePath, ClientTypePath},
        },
    },
    timestamp::Timestamp,
};
use lcp_types::{Any, Height, Time};
use light_client::{ClientKeeper, ClientReader};
use log::*;
use store::KVStore;

pub static NEXT_CLIENT_SEQUENCE: &str = "nextClientSequence";

pub struct Context<'a, 'e, S> {
    store: &'a mut S,
    ek: &'e dyn Signer,
    current_timestamp: Option<Time>,
}

impl<'a, 'e, S> Context<'a, 'e, S> {
    pub fn new(store: &'a mut S, ek: &'e dyn Signer) -> Self {
        Self {
            store,
            ek,
            current_timestamp: None,
        }
    }

    pub fn set_timestamp(&mut self, timestamp: Time) {
        self.current_timestamp = Some(timestamp)
    }

    pub fn get_enclave_key(&self) -> &'e dyn Signer {
        self.ek
    }
}

impl<'a, 'e, S: KVStore> ClientReader for Context<'a, 'e, S> {
    fn client_type(&self, client_id: &ClientId) -> Result<String, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientTypePath(client_id.clone())).as_bytes());
        if value.is_none() {
            return Err(ICS02Error::client_not_found(client_id.clone()));
        }
        Ok(String::from_utf8(value.unwrap()).unwrap())
    }

    fn client_state(&self, client_id: &ClientId) -> Result<Any, ICS02Error> {
        let value = self
            .store
            .get(format!("{}", ClientStatePath(client_id.clone())).as_bytes());
        if value.is_none() {
            return Err(ICS02Error::client_not_found(client_id.clone()));
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
                return Err(ICS02Error::consensus_state_not_found(
                    client_id.clone(),
                    height.try_into()?,
                ));
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

    fn as_ibc_client_reader(&self) -> &dyn IBCClientReader {
        self
    }
}

impl<'a, 'e, S: KVStore> IBCClientReader for Context<'a, 'e, S> {
    fn client_type(&self, client_id: &ClientId) -> Result<ClientType, ICS02Error> {
        let client_type = <Self as ClientReader>::client_type(&self, client_id)?;
        ClientType::from_str(&client_type)
    }

    fn client_state(&self, client_id: &ClientId) -> Result<AnyClientState, ICS02Error> {
        let client_state = <Self as ClientReader>::client_state(&self, client_id)?;
        AnyClientState::try_from(client_state)
    }

    fn consensus_state(
        &self,
        client_id: &ClientId,
        height: ICS02Height,
    ) -> Result<AnyConsensusState, ICS02Error> {
        let consensus_state =
            <Self as ClientReader>::consensus_state(&self, client_id, height.into())?;
        AnyConsensusState::try_from(consensus_state)
    }

    /// Similar to `consensus_state`, attempt to retrieve the consensus state,
    /// but return `None` if no state exists at the given height.
    fn maybe_consensus_state(
        &self,
        client_id: &ClientId,
        height: ICS02Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        use ibc::core::ics02_client::error::ErrorDetail;
        debug!("maybe_consensus_state: {:?}", height);
        match <Self as ClientReader>::consensus_state(&self, client_id, height.into()) {
            Ok(cs) => Ok(Some(cs.try_into()?)),
            Err(e) => match e.detail() {
                ErrorDetail::ConsensusStateNotFound(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    fn next_consensus_state(
        &self,
        _client_id: &ClientId,
        _height: ICS02Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        todo!()
    }

    fn prev_consensus_state(
        &self,
        _client_id: &ClientId,
        _height: ICS02Height,
    ) -> Result<Option<AnyConsensusState>, ICS02Error> {
        // TODO implement this
        Ok(None)
    }

    fn host_height(&self) -> ICS02Height {
        <Self as ClientReader>::host_height(&self)
            .try_into()
            .unwrap()
    }

    fn host_timestamp(&self) -> Timestamp {
        <Self as ClientReader>::host_timestamp(&self)
    }

    fn host_consensus_state(&self, _height: ICS02Height) -> Result<AnyConsensusState, ICS02Error> {
        todo!()
    }

    fn pending_host_consensus_state(&self) -> Result<AnyConsensusState, ICS02Error> {
        todo!()
    }

    fn client_counter(&self) -> Result<u64, ICS02Error> {
        <Self as ClientReader>::client_counter(&self)
    }
}

impl<'a, 'e, S: KVStore> ClientKeeper for Context<'a, 'e, S> {
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

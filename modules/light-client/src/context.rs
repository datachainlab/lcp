use crate::types::{Any, ClientId, Height, Time};
use crate::{errors::Error, prelude::*};
use commitments::StateID;
use lcp_types::store_key;
use store::KVStore;

pub trait HostContext {
    /// Returns the current timestamp of the local.
    fn host_timestamp(&self) -> Time;
}

pub trait ClientReader: KVStore {
    /// Returns `true` if the client exists in the store.
    fn client_exists(&self, client_id: &ClientId) -> bool {
        self.get(store_key::client_type(client_id.as_str()).as_bytes())
            .is_some()
    }

    /// Returns the ClientType for the given identifier `client_id`.
    fn client_type(&self, client_id: &ClientId) -> Result<String, Error> {
        let value = self.get(store_key::client_type(client_id.as_str()).as_bytes());
        if let Some(value) = value {
            Ok(String::from_utf8(value).unwrap())
        } else {
            Err(Error::client_type_not_found(client_id.clone()))
        }
    }

    /// Returns the ClientState for the given identifier `client_id`.
    fn client_state(&self, client_id: &ClientId) -> Result<Any, Error> {
        let value = self.get(store_key::client_state(client_id.as_str()).as_bytes());
        if let Some(value) = value {
            Ok(
                bincode::serde::decode_from_slice(&value, bincode::config::standard())
                    .unwrap()
                    .0,
            )
        } else {
            Err(Error::client_state_not_found(client_id.clone()))
        }
    }

    /// Retrieve the consensus state for the given client ID at the specified
    /// height.
    ///
    /// Returns an error if no such state exists.
    fn consensus_state(&self, client_id: &ClientId, height: &Height) -> Result<Any, Error> {
        let key = store_key::consensus_state(client_id.as_str(), height);
        let value = match self.get(key.as_bytes()) {
            Some(value) => value,
            None => {
                return Err(Error::consensus_state_not_found(client_id.clone(), *height));
            }
        };
        Ok(
            bincode::serde::decode_from_slice(&value, bincode::config::standard())
                .unwrap()
                .0,
        )
    }
}

pub trait ClientKeeper: ClientReader {
    /// Called upon successful client creation
    fn store_client_type(&mut self, client_id: ClientId, client_type: String) -> Result<(), Error> {
        self.set(
            store_key::client_type_bytes(client_id.as_str()),
            client_type.into_bytes(),
        );
        Ok(())
    }

    /// Called upon successful client creation and update
    fn store_any_client_state(
        &mut self,
        client_id: ClientId,
        client_state: Any,
    ) -> Result<(), Error> {
        let bz = bincode::serde::encode_to_vec(&client_state, bincode::config::standard()).unwrap();
        self.set(store_key::client_state_bytes(client_id.as_str()), bz);
        Ok(())
    }

    /// Called upon successful client creation and update
    fn store_any_consensus_state(
        &mut self,
        client_id: ClientId,
        height: Height,
        consensus_state: Any,
    ) -> Result<(), Error> {
        let bz =
            bincode::serde::encode_to_vec(&consensus_state, bincode::config::standard()).unwrap();
        self.set(
            store_key::consensus_state_bytes(client_id.as_str(), &height),
            bz,
        );
        Ok(())
    }

    /// Called upon successful client creation and update to index the state ID
    /// for the state at `height`. This keeps historical base validation compact:
    /// client_state remains latest-only while consensus_state and state_id are
    /// height-indexed.
    fn store_state_id(
        &mut self,
        client_id: ClientId,
        height: Height,
        state_id: StateID,
    ) -> Result<(), Error> {
        self.set(
            store_key::state_id_bytes(client_id.as_str(), &height),
            state_id.to_vec(),
        );
        Ok(())
    }
}

pub trait HostClientReader: HostContext + ClientReader {}

pub trait HostClientKeeper: HostClientReader + HostContext + ClientKeeper {}

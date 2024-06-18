use crate::types::{Any, ClientId, Height, Time};
use crate::{
    errors::Error,
    path::{ClientConsensusStatePath, ClientStatePath, ClientTypePath},
    prelude::*,
};
use store::KVStore;

pub trait HostContext {
    /// Returns the current timestamp of the local.
    fn host_timestamp(&self) -> Time;
}

pub trait ClientReader: KVStore {
    /// Returns `true` if the client exists in the store.
    fn client_exists(&self, client_id: &ClientId) -> bool {
        self.get(format!("{}", ClientTypePath::new(client_id)).as_bytes())
            .is_some()
    }

    /// Returns the ClientType for the given identifier `client_id`.
    fn client_type(&self, client_id: &ClientId) -> Result<String, Error> {
        let value = self.get(format!("{}", ClientTypePath::new(client_id)).as_bytes());
        if let Some(value) = value {
            Ok(String::from_utf8(value).unwrap())
        } else {
            Err(Error::client_type_not_found(client_id.clone()))
        }
    }

    /// Returns the ClientState for the given identifier `client_id`.
    fn client_state(&self, client_id: &ClientId) -> Result<Any, Error> {
        let value = self.get(format!("{}", ClientStatePath::new(client_id)).as_bytes());
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
        let path = ClientConsensusStatePath::new(client_id, height);
        let value = match self.get(format!("{}", path).as_bytes()) {
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
            format!("{}", ClientTypePath(client_id)).into_bytes(),
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
        self.set(
            format!("{}", ClientStatePath::new(&client_id)).into_bytes(),
            bz,
        );
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
        let path = ClientConsensusStatePath::new(&client_id, &height);
        self.set(format!("{}", path).into_bytes(), bz);
        Ok(())
    }
}

pub trait HostClientReader: HostContext + ClientReader {}

pub trait HostClientKeeper: HostClientReader + HostContext + ClientKeeper {}

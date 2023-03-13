use crate::{
    errors::Error,
    path::{ClientConsensusStatePath, ClientStatePath, ClientTypePath, NEXT_CLIENT_SEQUENCE},
    prelude::*,
};
use lcp_types::{Any, ClientId, Height, Time};
use store::KVStore;

pub trait HostContext {
    /// Returns the current timestamp of the local.
    fn host_timestamp(&self) -> Time;
}

pub trait ClientReader: KVStore {
    /// Returns the ClientType for the given identifier `client_id`.
    fn client_type(&self, client_id: &ClientId) -> Result<String, Error> {
        let value = self.get(format!("{}", ClientTypePath::new(client_id)).as_bytes());
        if value.is_none() {
            Err(Error::client_type_not_found(client_id.clone()))
        } else {
            Ok(String::from_utf8(value.unwrap()).unwrap())
        }
    }

    /// Returns the ClientState for the given identifier `client_id`.
    fn client_state(&self, client_id: &ClientId) -> Result<Any, Error> {
        let value = self.get(format!("{}", ClientStatePath::new(client_id)).as_bytes());
        if value.is_none() {
            Err(Error::client_state_not_found(client_id.clone()))
        } else {
            Ok(
                bincode::serde::decode_from_slice(&value.unwrap(), bincode::config::standard())
                    .unwrap()
                    .0,
            )
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
                return Err(Error::consensus_state_not_found(
                    client_id.clone(),
                    height.clone(),
                ));
            }
        };
        Ok(
            bincode::serde::decode_from_slice(&value, bincode::config::standard())
                .unwrap()
                .0,
        )
    }

    /// Returns a natural number, counting how many clients have been created thus far.
    /// The value of this counter should increase only via method `ClientKeeper::increase_client_counter`.
    fn client_counter(&self) -> Result<u64, Error> {
        match self.get(NEXT_CLIENT_SEQUENCE.as_bytes()) {
            Some(bz) => {
                let mut b: [u8; 8] = Default::default();
                b.copy_from_slice(&bz);
                Ok(u64::from_be_bytes(b))
            }
            None => Ok(0),
        }
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

    /// Called upon client creation.
    /// Increases the counter which keeps track of how many clients have been created.
    /// Should never fail.
    fn increase_client_counter(&mut self) {
        let next_counter = <Self as ClientReader>::client_counter(self).unwrap() + 1;
        self.set(
            NEXT_CLIENT_SEQUENCE.as_bytes().to_vec(),
            next_counter.to_be_bytes().to_vec(),
        );
    }
}

pub trait HostClientReader: HostContext + ClientReader {}

pub trait HostClientKeeper: HostContext + ClientKeeper {}

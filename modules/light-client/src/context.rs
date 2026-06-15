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
    ///
    /// This reads the "latest" client_state singleton, which is also the only
    /// entry that every existing ELC reader queries. The per-height history
    /// written by [`ClientKeeper::store_any_client_state`] is read via
    /// [`ClientReader::client_state_at_height`] instead.
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

    /// Returns the ClientState for the given identifier `client_id` at the
    /// specified `height`. Used by `verify_expected_base_state_in_tx` and the
    /// `query_client_at_height` RPC to resolve past anchors when an
    /// explicit-state batch is dispatched against a height that may no longer
    /// be the canonical tip (per-height client_state design).
    ///
    /// Falls back to the singleton `client_state(client_id)` if the per-height
    /// entry is absent (legacy compatibility for stores written before the
    /// per-height entries were introduced).
    fn client_state_at_height(&self, client_id: &ClientId, height: &Height) -> Result<Any, Error> {
        let key = store_key::client_state_at_height(client_id.as_str(), height);
        if let Some(value) = self.get(key.as_bytes()) {
            return Ok(
                bincode::serde::decode_from_slice(&value, bincode::config::standard())
                    .unwrap()
                    .0,
            );
        }
        // Legacy fallback: pre-D stores only have the singleton entry.
        self.client_state(client_id)
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

    /// Called upon successful client creation and update.
    ///
    /// Writes the same client_state bytes under two keys:
    /// (1) the singleton `client_state_bytes(client_id)` — preserves
    ///     "latest" semantics for every existing ELC reader that calls
    ///     [`ClientReader::client_state`].
    /// (2) the per-height `client_state_at_height_bytes(client_id, height)`
    ///     — enables `verify_expected_base_state_in_tx` and
    ///     `query_client_at_height` to resolve any past committed
    ///     anchor (per-height client_state design).
    fn store_any_client_state(
        &mut self,
        client_id: ClientId,
        height: Height,
        client_state: Any,
    ) -> Result<(), Error> {
        let bz = bincode::serde::encode_to_vec(&client_state, bincode::config::standard()).unwrap();
        self.set(store_key::client_state_bytes(client_id.as_str()), bz.clone());
        self.set(
            store_key::client_state_at_height_bytes(client_id.as_str(), &height),
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
        self.set(
            store_key::consensus_state_bytes(client_id.as_str(), &height),
            bz,
        );
        Ok(())
    }

    /// Called upon successful client creation and update to index the state ID
    /// for the state at `height`. With per-height client_state design in place,
    /// client_state is *also* height-indexed (via
    /// [`ClientKeeper::store_any_client_state`]); the three per-height tables
    /// — client_state, consensus_state, state_id — now share the same shape,
    /// which is what lets `verify_expected_base_state_in_tx` resolve a
    /// supplied speculative base byte-for-byte against the stored canonical
    /// at any committed past height.
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

#[cfg(test)]
mod tests {
    use super::*;
    use store::memory::MemStore;

    // Per-height client_state coexistence with the singleton (the per-height
    // client_state design). Tests live in `light-client` because the trait
    // default methods being exercised are defined here.
    impl ClientReader for MemStore {}
    impl ClientKeeper for MemStore {}

    fn any_cs(tag: &str) -> Any {
        Any::new(
            "/ibc.mock.ClientState".to_string(),
            tag.as_bytes().to_vec(),
        )
    }

    fn any_consensus(tag: &str) -> Any {
        Any::new(
            "/ibc.mock.ConsensusState".to_string(),
            tag.as_bytes().to_vec(),
        )
    }

    fn client_id() -> ClientId {
        use core::str::FromStr;
        ClientId::from_str("07-tendermint-0").unwrap()
    }

    #[test]
    fn store_any_client_state_writes_singleton_and_per_height() {
        let mut store = MemStore::default();
        let cid = client_id();
        let h = Height::new(0, 10);

        store
            .store_any_client_state(cid.clone(), h, any_cs("v10"))
            .unwrap();

        let singleton = store.client_state(&cid).unwrap();
        let at_height = store.client_state_at_height(&cid, &h).unwrap();
        assert_eq!(singleton, at_height, "singleton must equal per-height entry");
    }

    #[test]
    fn store_any_client_state_keeps_history_when_advancing() {
        let mut store = MemStore::default();
        let cid = client_id();
        let h1 = Height::new(0, 10);
        let h2 = Height::new(0, 11);

        store
            .store_any_client_state(cid.clone(), h1, any_cs("v10"))
            .unwrap();
        store
            .store_any_client_state(cid.clone(), h2, any_cs("v11"))
            .unwrap();

        // singleton tracks the latest write
        assert_eq!(store.client_state(&cid).unwrap(), any_cs("v11"));
        // per-height entries at both heights coexist
        assert_eq!(store.client_state_at_height(&cid, &h1).unwrap(), any_cs("v10"));
        assert_eq!(store.client_state_at_height(&cid, &h2).unwrap(), any_cs("v11"));
    }

    #[test]
    fn client_state_at_height_falls_back_to_singleton_when_entry_missing() {
        // Simulates a legacy (pre-D) store that has only the singleton.
        let mut store = MemStore::default();
        let cid = client_id();
        let legacy_height = Height::new(0, 42);

        // Write *only* the singleton, as a pre-D code path would have done.
        let bz = bincode::serde::encode_to_vec(
            &any_cs("legacy"),
            bincode::config::standard(),
        )
        .unwrap();
        store.set(store_key::client_state_bytes(cid.as_str()), bz);

        // Lookup at an arbitrary height resolves via the singleton fallback.
        let resolved = store
            .client_state_at_height(&cid, &legacy_height)
            .unwrap();
        assert_eq!(resolved, any_cs("legacy"));
    }

    #[test]
    fn consensus_and_state_id_are_height_indexed_independently_of_client_state() {
        let mut store = MemStore::default();
        let cid = client_id();
        let h = Height::new(0, 99);

        store
            .store_any_consensus_state(cid.clone(), h, any_consensus("c99"))
            .unwrap();
        store
            .store_state_id(cid.clone(), h, commitments::StateID::default())
            .unwrap();

        assert_eq!(
            store.consensus_state(&cid, &h).unwrap(),
            any_consensus("c99")
        );
        let id = store
            .get(store_key::state_id_bytes(cid.as_str(), &h).as_slice())
            .unwrap();
        assert_eq!(id, commitments::StateID::default().to_vec());
    }
}

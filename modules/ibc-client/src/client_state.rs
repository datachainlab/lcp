use crate::crypto::Address;
use crate::header::{Commitment, Header, UpdateClientHeader};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::core::{ics02_client::client_state::AnyClientState, ics24_host::identifier::ChainId};
use ibc::Height;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ClientState {
    pub latest_height: Height,
    pub mr_enclave: Vec<u8>,
    pub keys: Vec<Address>,
}

impl ClientState {
    pub fn contains(&self, key: &Address) -> bool {
        self.keys.contains(key)
    }

    pub fn with_header(mut self, header: &dyn Commitment) -> Self {
        if self.latest_height < header.height() {
            self.latest_height = header.height();
        }
        self
    }

    pub fn with_new_key(mut self, key: Address) -> Self {
        todo!()
        // assert!(!self.contains(&key));
        // self.keys.push(key);
        // self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions {}

impl ibc::core::ics02_client::client_state::ClientState for ClientState {
    type UpgradeOptions = UpgradeOptions;

    fn chain_id(&self) -> ChainId {
        todo!()
    }

    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn latest_height(&self) -> Height {
        self.latest_height
    }

    fn frozen_height(&self) -> Option<Height> {
        todo!()
    }

    fn upgrade(
        mut self,
        upgrade_height: Height,
        upgrade_options: UpgradeOptions,
        chain_id: ChainId,
    ) -> Self {
        todo!()
    }

    fn wrap_any(self) -> AnyClientState {
        todo!()
    }
}

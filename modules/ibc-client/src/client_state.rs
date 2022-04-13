use crate::public_key::Address;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::{ics02_client::client_state::AnyClientState, ics24_host::identifier::ChainId};
use ibc::Height;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ClientState {
    pub latest_height: Height,
    pub mrenclave: Vec<u8>,
    pub keys: Vec<Address>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions {}

impl ibc::core::ics02_client::client_state::ClientState for ClientState {
    type UpgradeOptions = UpgradeOptions;

    fn chain_id(&self) -> ChainId {
        todo!()
    }

    fn client_type(&self) -> ClientType {
        todo!()
    }

    fn latest_height(&self) -> Height {
        todo!()
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

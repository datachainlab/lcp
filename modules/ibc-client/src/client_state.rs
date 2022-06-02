use crate::crypto::Address;
use crate::header::{Commitment, Header, UpdateClientHeader};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::core::{ics02_client::client_state::AnyClientState, ics24_host::identifier::ChainId};
use ibc::Height;
use prost_types::Any;
use serde::{Deserialize, Serialize};
use std::vec::Vec;
use tendermint_proto::Protobuf;

pub const LCP_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ClientState";

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientState {
    pub latest_height: Height,
    pub mr_enclave: Vec<u8>,
    pub key_expiration: u128, // sec
    pub keys: Vec<(Address, u128)>,
}

impl ClientState {
    pub fn contains(&self, key: &Address) -> bool {
        self.keys.iter().find(|k| &k.0 == key).is_some()
    }

    pub fn with_header(mut self, header: &dyn Commitment) -> Self {
        if self.latest_height < header.height() {
            self.latest_height = header.height();
        }
        self
    }

    pub fn with_new_key(mut self, key: (Address, u128)) -> Self {
        assert!(!self.contains(&key.0));
        self.keys.push(key);
        self
    }
}

impl Protobuf<Any> for ClientState {}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        Any {
            type_url: LCP_CLIENT_STATE_TYPE_URL.to_string(),
            value: value
                .encode_vec()
                .expect("encoding to `Any` from `ClientState`"),
        }
    }
}

impl TryFrom<Any> for ClientState {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            "" => Err(Error::empty_client_state_response()),
            LCP_CLIENT_STATE_TYPE_URL => {
                Ok(ClientState::decode_vec(&raw.value).map_err(Error::decode_raw_client_state)?)
            }
            _ => Err(Error::unknown_client_state_type(raw.type_url)),
        }
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

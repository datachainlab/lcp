use crate::header::Commitment;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crypto::Address;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error;
use ibc::core::ics02_client::height::Height as ICS02Height;
use ibc::core::{ics02_client::client_state::AnyClientState, ics24_host::identifier::ChainId};
use lcp_proto::ibc::core::client::v1::Height as ProtoHeight;
use lcp_proto::ibc::lightclients::lcp::v1::ClientState as RawClientState;
use lcp_types::{Any, Height};
use prost::Message;
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};
use std::vec::Vec;
use tendermint_proto::Protobuf;

pub const LCP_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ClientState";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientState {
    pub latest_height: Height,
    pub mr_enclave: Vec<u8>,
    pub key_expiration: u128, // sec
    pub keys: Vec<(u128, Address)>,
}

impl ClientState {
    pub fn contains(&self, key: &Address) -> bool {
        self.keys.iter().find(|k| &k.1 == key).is_some()
    }

    pub fn with_header(mut self, header: &dyn Commitment) -> Self {
        if self.latest_height < header.height() {
            self.latest_height = header.height();
        }
        self
    }

    pub fn with_new_key(mut self, key: (u128, Address)) -> Self {
        assert!(!self.contains(&key.1));
        self.keys.push(key);
        self
    }
}

impl From<ClientState> for RawClientState {
    fn from(value: ClientState) -> Self {
        let keys = value
            .keys
            .iter()
            .map(|k| {
                let mut key = vec![0; 36];
                key[..16].copy_from_slice(k.0.to_be_bytes().as_slice());
                key[16..].copy_from_slice(&k.1 .0);
                key
            })
            .collect();
        RawClientState {
            latest_height: Some(ProtoHeight {
                revision_number: value.latest_height.revision_number(),
                revision_height: value.latest_height.revision_height(),
            }),
            mrenclave: value.mr_enclave,
            key_expiration: value.key_expiration as u64,
            keys,
        }
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        let height = raw.latest_height.unwrap();
        let keys = raw
            .keys
            .iter()
            .map(|k| {
                let ks = k.as_slice();
                let mut expiration: [u8; 16] = Default::default();
                expiration.copy_from_slice(&ks[..16]);
                (u128::from_be_bytes(expiration), Address::from(&ks[16..]))
            })
            .collect();
        Ok(ClientState {
            latest_height: Height::new(height.revision_number, height.revision_height),
            mr_enclave: raw.mrenclave,
            key_expiration: raw.key_expiration as u128,
            keys,
        })
    }
}

impl Protobuf<ProtoAny> for ClientState {}

impl From<ClientState> for ProtoAny {
    fn from(value: ClientState) -> Self {
        let value = RawClientState::try_from(value).expect("encoding to `Any` from `ClientState`");
        ProtoAny {
            type_url: LCP_CLIENT_STATE_TYPE_URL.to_string(),
            value: value.encode_to_vec(),
        }
    }
}

impl TryFrom<ProtoAny> for ClientState {
    type Error = Error;

    fn try_from(raw: ProtoAny) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            "" => Err(Error::empty_client_state_response()),
            LCP_CLIENT_STATE_TYPE_URL => {
                ClientState::try_from(RawClientState::decode(&*raw.value).unwrap())
            }
            _ => Err(Error::unknown_client_state_type(raw.type_url)),
        }
    }
}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        ProtoAny::from(value).into()
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

    fn latest_height(&self) -> ICS02Height {
        self.latest_height.try_into().unwrap()
    }

    fn frozen_height(&self) -> Option<ICS02Height> {
        todo!()
    }

    fn upgrade(
        mut self,
        upgrade_height: ICS02Height,
        upgrade_options: UpgradeOptions,
        chain_id: ChainId,
    ) -> Self {
        todo!()
    }

    fn wrap_any(self) -> AnyClientState {
        todo!()
    }
}

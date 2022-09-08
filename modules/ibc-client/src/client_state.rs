use crate::header::Commitment;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use core::time::Duration;
use crypto::Address;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error;
use ibc::core::ics02_client::height::Height as ICS02Height;
use ibc::core::{ics02_client::client_state::AnyClientState, ics24_host::identifier::ChainId};
use lcp_proto::ibc::core::client::v1::Height as ProtoHeight;
use lcp_proto::ibc::lightclients::lcp::v1::ClientState as RawClientState;
use lcp_types::{Any, Height, Time};
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
    pub key_expiration: Duration,
    pub keys: Vec<(Address, Time)>,
}

impl ClientState {
    pub fn contains(&self, key: &Address) -> bool {
        self.keys.iter().find(|k| &k.0 == key).is_some()
    }

    pub fn is_active_key(&self, current_time: Time, key: &Address) -> bool {
        let expired_time = (current_time - self.key_expiration).unwrap();
        match self.keys.iter().find(|k| &k.0 == key) {
            Some(entry) => entry.1 > expired_time,
            None => false,
        }
    }

    pub fn with_header(mut self, header: &dyn Commitment) -> Self {
        if self.latest_height < header.height() {
            self.latest_height = header.height();
        }
        self
    }

    pub fn with_new_key(mut self, entry: (Address, Time)) -> Self {
        assert!(!self.contains(&entry.0));
        self.keys.push(entry);
        self
    }
}

impl From<ClientState> for RawClientState {
    fn from(value: ClientState) -> Self {
        let mut client_state = RawClientState {
            latest_height: Some(ProtoHeight {
                revision_number: value.latest_height.revision_number(),
                revision_height: value.latest_height.revision_height(),
            }),
            mrenclave: value.mr_enclave,
            key_expiration: value.key_expiration.as_secs(),
            keys: Vec::with_capacity(value.keys.len()),
            attestation_times: Vec::with_capacity(value.keys.len()),
        };
        value.keys.into_iter().for_each(|k| {
            client_state.keys.push(k.0.into());
            client_state
                .attestation_times
                .push(k.1.as_unix_timestamp_secs());
        });
        client_state
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        let height = raw.latest_height.unwrap();
        let keys = raw
            .keys
            .iter()
            .zip(raw.attestation_times.iter())
            .map(|entry| {
                (
                    Address::from(entry.0.as_slice()),
                    Time::from_unix_timestamp_secs(*entry.1).unwrap(),
                )
            })
            .collect();
        Ok(ClientState {
            latest_height: Height::new(height.revision_number, height.revision_height),
            mr_enclave: raw.mrenclave,
            key_expiration: Duration::from_secs(raw.key_expiration),
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

use crate::errors::Error;
use crate::header::Commitment;
use crate::prelude::*;
use core::time::Duration;
use crypto::Address;
use ibc_proto::protobuf::Protobuf;
use lcp_proto::ibc::core::client::v1::Height as ProtoHeight;
use lcp_proto::ibc::lightclients::lcp::v1::ClientState as RawClientState;
use lcp_types::{Any, Height, Time};
use prost::Message;
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};

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
        self.keys.iter().any(|k| &k.0 == key)
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
            allowed_quote_statuses: Default::default(),
            allowed_advisory_ids: Default::default(),
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
            LCP_CLIENT_STATE_TYPE_URL => Ok(ClientState::try_from(
                RawClientState::decode(&*raw.value).unwrap(),
            )?),
            _ => Err(Error::unexpected_client_type(raw.type_url)),
        }
    }
}

impl TryFrom<Any> for ClientState {
    type Error = Error;

    fn try_from(any: Any) -> Result<Self, Self::Error> {
        TryFrom::<ProtoAny>::try_from(any.into())
    }
}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        ProtoAny::from(value).try_into().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions {}

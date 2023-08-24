use crate::errors::Error;
use crate::message::CommitmentReader;
use crate::prelude::*;
use core::time::Duration;
use light_client::types::proto::{
    ibc::{
        core::client::v1::Height as ProtoHeight,
        lightclients::lcp::v1::ClientState as RawClientState,
    },
    protobuf::Protobuf,
};
use light_client::types::{Any, Height};
use prost::Message;
use serde::{Deserialize, Serialize};

pub const LCP_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ClientState";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientState {
    pub latest_height: Height,
    pub mr_enclave: Vec<u8>,
    pub key_expiration: Duration,
}

impl ClientState {
    pub fn with_header<C: CommitmentReader>(mut self, header: &C) -> Self {
        if self.latest_height < header.height() {
            self.latest_height = header.height();
        }
        self
    }
}

impl From<ClientState> for RawClientState {
    fn from(value: ClientState) -> Self {
        RawClientState {
            latest_height: Some(ProtoHeight {
                revision_number: value.latest_height.revision_number(),
                revision_height: value.latest_height.revision_height(),
            }),
            mrenclave: value.mr_enclave,
            key_expiration: value.key_expiration.as_secs(),
            allowed_quote_statuses: Default::default(),
            allowed_advisory_ids: Default::default(),
        }
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        let height = raw.latest_height.unwrap();
        Ok(ClientState {
            latest_height: Height::new(height.revision_number, height.revision_height),
            mr_enclave: raw.mrenclave,
            key_expiration: Duration::from_secs(raw.key_expiration),
        })
    }
}

impl Protobuf<Any> for ClientState {}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        let value = RawClientState::try_from(value).expect("encoding to `Any` from `ClientState`");
        Any::new(LCP_CLIENT_STATE_TYPE_URL.to_string(), value.encode_to_vec())
    }
}

impl TryFrom<Any> for ClientState {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_CLIENT_STATE_TYPE_URL => Ok(ClientState::try_from(
                RawClientState::decode(&*raw.value).unwrap(),
            )?),
            type_url => Err(Error::unexpected_client_type(type_url.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions {}

use crate::errors::Error;
use crate::prelude::*;
use core::time::Duration;
use crypto::Address;
use light_client::commitments::UpdateStateProxyMessage;
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

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientState {
    pub mr_enclave: Vec<u8>,
    pub key_expiration: Duration,
    pub latest_height: Height,
    pub frozen: bool,
    pub operators: Vec<Address>,
    pub operators_nonce: u64,
    pub operators_threshold_numerator: u64,
    pub operators_threshold_denominator: u64,
    pub zkdcap_risc0_image_id: Option<[u8; 32]>,
}

impl ClientState {
    pub fn with_header(mut self, header: &UpdateStateProxyMessage) -> Self {
        if self.latest_height < header.post_height {
            self.latest_height = header.post_height;
        }
        self
    }

    pub fn with_frozen(mut self) -> Self {
        self.frozen = true;
        self
    }

    pub fn with_operators(
        mut self,
        operators: Vec<Address>,
        nonce: u64,
        threshold_numerator: u64,
        threshold_denominator: u64,
    ) -> Self {
        self.operators = operators;
        self.operators_nonce = nonce;
        self.operators_threshold_numerator = threshold_numerator;
        self.operators_threshold_denominator = threshold_denominator;
        self
    }
}

impl From<ClientState> for RawClientState {
    fn from(value: ClientState) -> Self {
        RawClientState {
            mrenclave: value.mr_enclave,
            key_expiration: value.key_expiration.as_secs(),
            frozen: value.frozen,
            latest_height: Some(ProtoHeight {
                revision_number: value.latest_height.revision_number(),
                revision_height: value.latest_height.revision_height(),
            }),
            allowed_quote_statuses: Default::default(),
            allowed_advisory_ids: Default::default(),
            operators: Default::default(),
            operators_nonce: 0,
            operators_threshold_numerator: 0,
            operators_threshold_denominator: 0,
            zkdcap_risc0_image_id: value.zkdcap_risc0_image_id.unwrap_or_default().to_vec(),
        }
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        let height = raw.latest_height.unwrap();
        Ok(ClientState {
            mr_enclave: raw.mrenclave,
            key_expiration: Duration::from_secs(raw.key_expiration),
            frozen: raw.frozen,
            latest_height: Height::new(height.revision_number, height.revision_height),
            operators: raw
                .operators
                .into_iter()
                .map(|addr| Address::try_from(addr.as_slice()))
                .collect::<Result<_, _>>()?,
            operators_nonce: raw.operators_nonce,
            operators_threshold_numerator: raw.operators_threshold_numerator,
            operators_threshold_denominator: raw.operators_threshold_denominator,
            zkdcap_risc0_image_id: if raw.zkdcap_risc0_image_id.is_empty() {
                None
            } else {
                Some(
                    <[u8; 32]>::try_from(raw.zkdcap_risc0_image_id.as_slice()).map_err(|_| {
                        Error::invalid_zkdcap_risc0_image_id(raw.zkdcap_risc0_image_id)
                    })?,
                )
            },
        })
    }
}

impl Protobuf<Any> for ClientState {}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        let value = RawClientState::from(value);
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

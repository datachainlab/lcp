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

/// The type URL for the client state protobuf message
pub const LCP_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ClientState";

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientState {
    pub mr_enclave: Vec<u8>,
    pub key_expiration: Duration,
    pub latest_height: Height,
    pub frozen: bool,
    pub allowed_quote_statuses: Vec<String>,
    pub allowed_advisory_ids: Vec<String>,
    pub operators: Vec<Address>,
    pub operators_nonce: u64,
    pub operators_threshold_numerator: u64,
    pub operators_threshold_denominator: u64,
    pub current_tcb_evaluation_data_number: u32,
    pub tcb_evaluation_data_number_update_grace_period: u32,
    pub next_tcb_evaluation_data_number: u32,
    pub next_tcb_evaluation_data_number_update_time: u64,
    pub zkdcap_verifier_infos: Vec<ZKDCAPVerifierInfo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ZKVMType {
    #[default]
    Unspecified,
    Risc0,
}

impl ZKVMType {
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Unspecified),
            1 => Ok(Self::Risc0),
            _ => Err(Error::invalid_zkdcap_verifier_info(vec![value])),
        }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Unspecified => 0,
            Self::Risc0 => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ZKDCAPVerifierInfo {
    #[default]
    Unspecified,
    Risc0([u8; 32]),
}

impl ZKDCAPVerifierInfo {
    pub fn as_type(&self) -> ZKVMType {
        match self {
            Self::Unspecified => ZKVMType::Unspecified,
            Self::Risc0(_) => ZKVMType::Risc0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.is_empty() {
            return Ok(Self::Unspecified);
        }
        let zkvm_type = ZKVMType::from_u8(bytes[0])?;
        match zkvm_type {
            ZKVMType::Unspecified => Ok(Self::Unspecified),
            ZKVMType::Risc0 => {
                if bytes.len() != 64 {
                    return Err(Error::invalid_zkdcap_verifier_info(bytes.to_vec()));
                }
                let mut image_id = [0u8; 32];
                image_id.copy_from_slice(&bytes[32..]);
                Ok(Self::Risc0(image_id))
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Unspecified => vec![0],
            Self::Risc0(image_id) => {
                let mut bytes = vec![ZKVMType::Risc0.to_u8()];
                bytes.extend_from_slice([0u8; 31].as_ref());
                bytes.extend_from_slice(image_id);
                bytes
            }
        }
    }
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
            current_tcb_evaluation_data_number: value.current_tcb_evaluation_data_number,
            tcb_evaluation_data_number_update_grace_period: value
                .tcb_evaluation_data_number_update_grace_period,
            next_tcb_evaluation_data_number: value.next_tcb_evaluation_data_number,
            next_tcb_evaluation_data_number_update_time: value
                .next_tcb_evaluation_data_number_update_time,
            zkdcap_verifier_infos: value
                .zkdcap_verifier_infos
                .iter()
                .map(|info| info.to_bytes())
                .collect(),
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
            allowed_quote_statuses: raw.allowed_quote_statuses,
            allowed_advisory_ids: raw.allowed_advisory_ids,
            operators: raw
                .operators
                .into_iter()
                .map(|addr| Address::try_from(addr.as_slice()))
                .collect::<Result<_, _>>()?,
            operators_nonce: raw.operators_nonce,
            operators_threshold_numerator: raw.operators_threshold_numerator,
            operators_threshold_denominator: raw.operators_threshold_denominator,
            current_tcb_evaluation_data_number: raw.current_tcb_evaluation_data_number,
            tcb_evaluation_data_number_update_grace_period: raw
                .tcb_evaluation_data_number_update_grace_period,
            next_tcb_evaluation_data_number: raw.next_tcb_evaluation_data_number,
            next_tcb_evaluation_data_number_update_time: raw
                .next_tcb_evaluation_data_number_update_time,
            zkdcap_verifier_infos: raw
                .zkdcap_verifier_infos
                .into_iter()
                .map(|bytes| ZKDCAPVerifierInfo::from_bytes(&bytes))
                .collect::<Result<_, _>>()?,
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

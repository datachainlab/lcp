use super::bytes_to_bytes32;
use crate::context::ValidationContext;
use crate::encoder::{EthABIEmittedState, EthABIEncoder, EthABIHeight};
use crate::prelude::*;
use crate::{Error, StateID};
use core::fmt::Display;
use lcp_types::{Any, Height, Time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateClientMessage {
    pub prev_height: Option<Height>,
    pub prev_state_id: Option<StateID>,
    pub post_height: Height,
    pub post_state_id: StateID,
    pub timestamp: Time,
    pub context: ValidationContext,
    pub emitted_states: Vec<EmittedState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmittedState(pub Height, pub Any);

impl Display for UpdateClientMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "UpdateClient(prev_height: {}, prev_state_id: {}, post_height: {}, post_state_id: {}, timestamp: {}, context: {}, emitted_states: {})",
            self.prev_height.as_ref().map_or("None".to_string(), |h| h.to_string()),
            self.prev_state_id.as_ref().map_or("None".to_string(), |id| id.to_string()),
            self.post_height,
            self.post_state_id,
            self.timestamp,
            self.context,
            self.emitted_states.len(),
        )
    }
}

/// the struct is encoded as a tuple of 7 elements
pub(crate) struct EthABIUpdateClientMessage {
    pub prev_height: EthABIHeight,               // (u64, u64)
    pub prev_state_id: ethabi::FixedBytes,       // bytes32
    pub post_height: EthABIHeight,               // (u64, u64)
    pub post_state_id: ethabi::FixedBytes,       // bytes32
    pub timestamp: ethabi::Uint,                 // u128
    pub context: ethabi::Bytes,                  // bytes
    pub emitted_states: Vec<EthABIEmittedState>, // [((u64, u64), bytes)]
}

impl EthABIUpdateClientMessage {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::Tuple(self.prev_height.into()),
            Token::FixedBytes(self.prev_state_id),
            Token::Tuple(self.post_height.into()),
            Token::FixedBytes(self.post_state_id),
            Token::Uint(self.timestamp),
            Token::Bytes(self.context),
            Token::Array(
                self.emitted_states
                    .into_iter()
                    .map(|v| Token::Tuple(v.into()))
                    .collect(),
            ),
        ])])
    }

    pub fn decode(bz: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::FixedBytes(32),
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::FixedBytes(32),
                ParamType::Uint(64),
                ParamType::Bytes,
                ParamType::Array(Box::new(ParamType::Tuple(vec![
                    ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                    ParamType::Bytes,
                ]))),
            ])],
            bz,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 7
        assert!(tuple.len() == 7);
        let mut values = tuple.into_iter();
        Ok(Self {
            prev_height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            prev_state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
            post_height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            post_state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
            timestamp: values.next().unwrap().into_uint().unwrap(),
            context: values.next().unwrap().into_bytes().unwrap(),
            emitted_states: values
                .next()
                .unwrap()
                .into_array()
                .unwrap()
                .into_iter()
                .map(|v| EthABIEmittedState::try_from(v.into_tuple().unwrap()))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl From<UpdateClientMessage> for EthABIUpdateClientMessage {
    fn from(value: UpdateClientMessage) -> Self {
        use ethabi::*;
        Self {
            prev_height: value.prev_height.into(),
            prev_state_id: FixedBytes::from(
                value.prev_state_id.unwrap_or_default().to_vec().as_slice(),
            ),
            post_height: value.post_height.into(),
            post_state_id: FixedBytes::from(value.post_state_id.to_vec().as_slice()),
            timestamp: Uint::from(value.timestamp.as_unix_timestamp_nanos()),
            context: value.context.ethabi_encode(),
            emitted_states: value
                .emitted_states
                .into_iter()
                .map(EthABIEmittedState::from)
                .collect(),
        }
    }
}

impl TryFrom<EthABIUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(value: EthABIUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_height: value.prev_height.into(),
            prev_state_id: bytes_to_bytes32(value.prev_state_id)?.map(StateID::from),
            post_height: value.post_height.into(),
            post_state_id: value.post_state_id.as_slice().try_into()?,
            timestamp: Time::from_unix_timestamp_nanos(value.timestamp.as_u128())?,
            context: ValidationContext::ethabi_decode(value.context.as_slice())?,
            emitted_states: value
                .emitted_states
                .into_iter()
                .map(EmittedState::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl EthABIEncoder for UpdateClientMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIUpdateClientMessage>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIUpdateClientMessage::decode(bz).and_then(|v| v.try_into())
    }
}

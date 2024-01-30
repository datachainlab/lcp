use crate::context::ValidationContext;
use crate::encoder::{EthABIEmittedState, EthABIEncoder, EthABIHeight};
use crate::prelude::*;
use crate::{Error, StateID};
use alloy_sol_types::{private::B256, sol, SolValue};
use core::fmt::Display;
use lcp_types::{Any, Height, Time};
use prost::Message;
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

impl Display for EmittedState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "EmittedState(height: {}, state: {})",
            self.0,
            hex::encode(self.1.encode_to_vec())
        )
    }
}

impl UpdateClientMessage {
    pub fn aggregate(self, other: Self) -> Result<Self, Error> {
        if self.post_state_id != other.prev_state_id.unwrap_or_default() {
            return Err(Error::message_aggregation_failed(format!(
                "invalid prev_state_id: expected={} actual={}",
                self.post_state_id,
                other.prev_state_id.unwrap_or_default()
            )));
        }
        if self.post_height != other.prev_height.unwrap_or_default() {
            return Err(Error::message_aggregation_failed(format!(
                "invalid prev_height: expected={} actual={}",
                self.post_height,
                other.prev_height.unwrap_or_default()
            )));
        }
        Ok(Self {
            prev_height: self.prev_height,
            prev_state_id: self.prev_state_id,
            post_height: other.post_height,
            post_state_id: other.post_state_id,
            timestamp: other.timestamp,
            context: self.context.aggregate(other.context)?,
            emitted_states: [self.emitted_states, other.emitted_states].concat(),
        })
    }
}

impl Display for UpdateClientMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "UpdateClient(prev_height: {}, prev_state_id: {}, post_height: {}, post_state_id: {}, timestamp: {}, context: {}, emitted_states: [{}])",
            self.prev_height.as_ref().map_or("None".to_string(), |h| h.to_string()),
            self.prev_state_id.as_ref().map_or("None".to_string(), |id| id.to_string()),
            self.post_height,
            self.post_state_id,
            self.timestamp.as_unix_timestamp_nanos(),
            self.context,
            self.emitted_states.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
        )
    }
}

/// Aggregate a list of messages into a single message
pub fn aggregate_messages(
    messages: Vec<UpdateClientMessage>,
) -> Result<UpdateClientMessage, Error> {
    if messages.is_empty() {
        return Err(Error::message_aggregation_failed(
            "cannot aggregate empty messages".to_string(),
        ));
    }
    let mut messages = messages.into_iter();
    let mut message = messages.next().unwrap();
    for m in messages {
        message = message.aggregate(m)?;
    }
    Ok(message)
}

sol! {
    struct EthABIUpdateClientMessage {
        EthABIHeight prev_height;
        bytes32 prev_state_id;
        EthABIHeight post_height;
        bytes32 post_state_id;
        uint128 timestamp;
        bytes context;
        EthABIEmittedState[] emitted_states;
    }
}

impl From<UpdateClientMessage> for EthABIUpdateClientMessage {
    fn from(msg: UpdateClientMessage) -> Self {
        Self {
            prev_height: msg.prev_height.into(),
            prev_state_id: B256::from_slice(
                msg.prev_state_id.unwrap_or_default().to_vec().as_slice(),
            ),
            post_height: msg.post_height.into(),
            post_state_id: B256::from_slice(msg.post_state_id.to_vec().as_slice()),
            timestamp: msg.timestamp.as_unix_timestamp_nanos(),
            context: msg.context.ethabi_encode(),
            emitted_states: msg
                .emitted_states
                .into_iter()
                .map(EthABIEmittedState::from)
                .collect(),
        }
    }
}

impl TryFrom<EthABIUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(msg: EthABIUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_height: msg.prev_height.into(),
            prev_state_id: (!msg.prev_state_id.is_zero())
                .then_some(StateID::from(msg.prev_state_id.0)),
            post_height: msg.post_height.into(),
            post_state_id: msg.post_state_id.as_slice().try_into()?,
            timestamp: Time::from_unix_timestamp_nanos(msg.timestamp)?,
            context: ValidationContext::ethabi_decode(msg.context.as_slice())?,
            emitted_states: msg
                .emitted_states
                .into_iter()
                .map(EmittedState::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl EthABIEncoder for UpdateClientMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIUpdateClientMessage>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIUpdateClientMessage::abi_decode(bz, true)?.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TrustingPeriodContext;
    use core::time::Duration;

    #[test]
    fn test_update_client_message_aggregation() {
        {
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(2, 2)),
                prev_state_id: Some(StateID::from([2u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let expected = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            assert_eq!(aggregate_messages(vec![msg0, msg1]).unwrap(), expected);
        }
        {
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![EmittedState(
                    Height::new(1, 1),
                    Any::new("/foo".to_string(), vec![1u8; 32]),
                )],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(2, 2)),
                prev_state_id: Some(StateID::from([2u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![EmittedState(
                    Height::new(2, 2),
                    Any::new("/bar".to_string(), vec![2u8; 32]),
                )],
            };
            let expected = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![
                    EmittedState(
                        Height::new(1, 1),
                        Any::new("/foo".to_string(), vec![1u8; 32]),
                    ),
                    EmittedState(
                        Height::new(2, 2),
                        Any::new("/bar".to_string(), vec![2u8; 32]),
                    ),
                ],
            };
            assert_eq!(aggregate_messages(vec![msg0, msg1]).unwrap(), expected);
        }
        {
            // trusting period aggregation
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: TrustingPeriodContext::new(
                    Duration::from_secs(1),
                    Duration::from_secs(2),
                    Time::from_unix_timestamp_nanos(1).unwrap(),
                    Time::from_unix_timestamp_nanos(2).unwrap(),
                )
                .into(),
                emitted_states: vec![],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(2, 2)),
                prev_state_id: Some(StateID::from([2u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: TrustingPeriodContext::new(
                    Duration::from_secs(1),
                    Duration::from_secs(2),
                    Time::from_unix_timestamp_nanos(2).unwrap(),
                    Time::from_unix_timestamp_nanos(3).unwrap(),
                )
                .into(),
                emitted_states: vec![],
            };
            let expected = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: TrustingPeriodContext::new(
                    Duration::from_secs(1),
                    Duration::from_secs(2),
                    Time::from_unix_timestamp_nanos(2).unwrap(),
                    Time::from_unix_timestamp_nanos(2).unwrap(),
                )
                .into(),
                emitted_states: vec![],
            };
            assert_eq!(aggregate_messages(vec![msg0, msg1]).unwrap(), expected);
        }
        {
            // invalid prev_state_id
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(2, 2)),
                prev_state_id: Some(StateID::from([3u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            assert!(msg0.aggregate(msg1).is_err());
        }
        {
            // invalid prev_height
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(3, 3)),
                prev_state_id: Some(StateID::from([2u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            assert!(msg0.aggregate(msg1).is_err());
        }
        {
            // empty messages
            assert!(aggregate_messages(vec![]).is_err());
        }
        {
            // single message
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            assert_eq!(aggregate_messages(vec![msg0.clone()]).unwrap(), msg0);
        }
        {
            // three messages
            let msg0 = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(2, 2),
                post_state_id: StateID::from([2u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(1).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let msg1 = UpdateClientMessage {
                prev_height: Some(Height::new(2, 2)),
                prev_state_id: Some(StateID::from([2u8; 32])),
                post_height: Height::new(3, 3),
                post_state_id: StateID::from([3u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(2).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let msg2 = UpdateClientMessage {
                prev_height: Some(Height::new(3, 3)),
                prev_state_id: Some(StateID::from([3u8; 32])),
                post_height: Height::new(4, 4),
                post_state_id: StateID::from([4u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(3).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            let expected = UpdateClientMessage {
                prev_height: Some(Height::new(1, 1)),
                prev_state_id: Some(StateID::from([1u8; 32])),
                post_height: Height::new(4, 4),
                post_state_id: StateID::from([4u8; 32]),
                timestamp: Time::from_unix_timestamp_nanos(3).unwrap(),
                context: ValidationContext::default(),
                emitted_states: vec![],
            };
            assert_eq!(
                aggregate_messages(vec![msg0, msg1, msg2]).unwrap(),
                expected
            );
        }
    }
}

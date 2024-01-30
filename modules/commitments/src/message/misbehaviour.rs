use crate::{encoder::EthABIHeight, prelude::*, Error, EthABIEncoder, StateID, ValidationContext};
use alloy_sol_types::{private::B256, sol, SolValue};
use core::fmt::Display;
use lcp_types::{Any, Height};
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MisbehaviourMessage {
    pub prev_states: Vec<PrevState>,
    pub context: ValidationContext,
    pub client_message: Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrevState {
    pub height: Height,
    pub state_id: StateID,
}

impl Display for MisbehaviourMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Misbehaviour(prev_states: {:?}, context: {}, client_message: {:?})",
            self.prev_states, self.context, self.client_message
        )
    }
}

sol! {
    struct EthABIPrevState {
        EthABIHeight height;
        bytes32 state_id;
    }

    struct EthABIMisbehaviourMessage {
        EthABIPrevState[] prev_states;
        bytes context;
        bytes client_message;
    }
}

impl From<MisbehaviourMessage> for EthABIMisbehaviourMessage {
    fn from(msg: MisbehaviourMessage) -> Self {
        Self {
            prev_states: msg
                .prev_states
                .into_iter()
                .map(|v| EthABIPrevState {
                    height: v.height.into(),
                    state_id: B256::from_slice(v.state_id.to_vec().as_slice()),
                })
                .collect(),
            context: msg.context.ethabi_encode(),
            client_message: msg.client_message.encode_to_vec(),
        }
    }
}

impl TryFrom<EthABIMisbehaviourMessage> for MisbehaviourMessage {
    type Error = Error;

    fn try_from(msg: EthABIMisbehaviourMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_states: msg
                .prev_states
                .into_iter()
                .map(|v| PrevState {
                    height: v.height.into(),
                    state_id: StateID::from(v.state_id.0),
                })
                .collect(),
            context: ValidationContext::ethabi_decode(msg.context.as_slice())?,
            client_message: Any::decode(msg.client_message.as_slice())
                .map_err(Error::proto_decode_error)?,
        })
    }
}

impl EthABIEncoder for MisbehaviourMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIMisbehaviourMessage>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIMisbehaviourMessage::abi_decode(bz, true)?.try_into()
    }
}

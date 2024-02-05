use crate::{encoder::EthABIHeight, prelude::*, Error, EthABIEncoder, StateID, ValidationContext};
use alloy_sol_types::{private::B256, sol, SolValue};
use core::fmt::Display;
use lcp_types::{Any, Height};
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MisbehaviourProxyMessage {
    pub prev_states: Vec<PrevState>,
    pub context: ValidationContext,
    pub client_message: Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrevState {
    pub height: Height,
    pub state_id: StateID,
}

impl MisbehaviourProxyMessage {
    pub fn validate(&self) -> Result<(), Error> {
        if self.prev_states.is_empty() {
            return Err(Error::empty_prev_states());
        }
        Ok(())
    }
}

impl Display for MisbehaviourProxyMessage {
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

    struct EthABIMisbehaviourProxyMessage {
        EthABIPrevState[] prev_states;
        bytes context;
        bytes client_message;
    }
}

impl Display for EthABIPrevState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "EthABIPrevState(height_revision_number: {} height_revision_height: {} state_id: 0x{})",
            self.height.revision_number, self.height.revision_height, self.state_id
        )
    }
}

impl Display for EthABIMisbehaviourProxyMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "EthABIMisbehaviourProxyMessage(prev_states: [{}], context: 0x{}, client_message: 0x{})",
            self.prev_states.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "), 
            hex::encode(&self.context), hex::encode(&self.client_message)
        )
    }
}

impl From<MisbehaviourProxyMessage> for EthABIMisbehaviourProxyMessage {
    fn from(msg: MisbehaviourProxyMessage) -> Self {
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

impl TryFrom<EthABIMisbehaviourProxyMessage> for MisbehaviourProxyMessage {
    type Error = Error;

    fn try_from(msg: EthABIMisbehaviourProxyMessage) -> Result<Self, Self::Error> {
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

impl EthABIEncoder for MisbehaviourProxyMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIMisbehaviourProxyMessage>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIMisbehaviourProxyMessage::abi_decode(bz, true)?.try_into()
    }
}

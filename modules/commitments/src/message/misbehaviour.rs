use crate::{encoder::EthABIHeight, prelude::*, Error, EthABIEncoder, StateID, ValidationContext};
use core::fmt::Display;
use ethabi::FixedBytes;
use lcp_types::{Any, Height};
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MisbehaviourMessage {
    pub prev_height: Height,
    pub prev_state_id: StateID,
    pub context: ValidationContext,
    pub client_message: Any,
}

impl Display for MisbehaviourMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "(prev_height: {}, prev_state_id: {}, context: {} client_message: {:?})",
            self.prev_height, self.prev_state_id, self.context, self.client_message,
        )
    }
}

pub(crate) struct EthABIMisbehaviourMessage {
    pub prev_height: EthABIHeight,         // (u64, u64)
    pub prev_state_id: ethabi::FixedBytes, // bytes32
    pub context: ethabi::Bytes,            // bytes
    pub client_message: ethabi::Bytes,     // bytes
}

impl EthABIMisbehaviourMessage {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::Tuple(self.prev_height.into()),
            Token::FixedBytes(self.prev_state_id),
            Token::Bytes(self.context),
        ])])
    }

    pub fn decode(bz: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::FixedBytes(32),
                ParamType::Bytes,
                ParamType::Bytes,
            ])],
            bz,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 7
        assert!(tuple.len() == 4);
        let mut values = tuple.into_iter();
        Ok(Self {
            prev_height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            prev_state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
            context: values.next().unwrap().into_bytes().unwrap(),
            client_message: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

impl From<MisbehaviourMessage> for EthABIMisbehaviourMessage {
    fn from(value: MisbehaviourMessage) -> Self {
        Self {
            prev_height: value.prev_height.into(),
            prev_state_id: FixedBytes::from(value.prev_state_id.to_vec().as_slice()),
            context: value.context.ethabi_encode(),
            client_message: value.client_message.encode_to_vec(),
        }
    }
}

impl TryFrom<EthABIMisbehaviourMessage> for MisbehaviourMessage {
    type Error = Error;

    fn try_from(value: EthABIMisbehaviourMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_height: value.prev_height.into(),
            prev_state_id: value.prev_state_id.as_slice().try_into()?,
            context: ValidationContext::ethabi_decode(value.context.as_slice())?,
            client_message: Any::decode(value.client_message.as_slice())
                .map_err(Error::proto_decode_error)?,
        })
    }
}

impl EthABIEncoder for MisbehaviourMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIMisbehaviourMessage>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIMisbehaviourMessage::decode(bz).and_then(|v| v.try_into())
    }
}

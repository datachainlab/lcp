use super::bytes_to_bytes32;
use crate::encoder::{EthABIEncoder, EthABIHeight};
use crate::prelude::*;
use crate::{Error, StateID};
use core::fmt::Display;
use lcp_types::Height;
use serde::{Deserialize, Serialize};

pub type CommitmentPrefix = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyMembershipMessage {
    pub prefix: CommitmentPrefix,
    pub path: String,
    pub value: Option<[u8; 32]>,
    pub height: Height,
    pub state_id: StateID,
}

impl Display for VerifyMembershipMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "VerifyMembership(prefix: {:?}, path: {}, value: {}, height: {}, state_id: {})",
            self.prefix,
            self.path,
            self.value.map_or("None".to_string(), |v| hex::encode(&v)),
            self.height,
            self.state_id,
        )
    }
}

pub(crate) struct EthABIVerifyMembershipMessage {
    prefix: ethabi::Bytes,        // bytes
    path: ethabi::Bytes,          // bytes
    value: ethabi::FixedBytes,    // bytes32
    height: EthABIHeight,         // (uint64, uint64)
    state_id: ethabi::FixedBytes, // bytes32
}

impl EthABIVerifyMembershipMessage {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::Bytes(self.prefix),
            Token::Bytes(self.path),
            Token::FixedBytes(self.value),
            Token::Tuple(self.height.into()),
            Token::FixedBytes(self.state_id),
        ])])
    }

    pub fn decode(bz: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::Bytes,
                ParamType::Bytes,
                ParamType::FixedBytes(32),
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::FixedBytes(32),
            ])],
            bz,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 5
        assert!(tuple.len() == 5);
        let mut values = tuple.into_iter();
        Ok(Self {
            prefix: values.next().unwrap().into_bytes().unwrap(),
            path: values.next().unwrap().into_bytes().unwrap().to_vec(),
            value: values.next().unwrap().into_fixed_bytes().unwrap(),
            height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
        })
    }
}

impl From<VerifyMembershipMessage> for EthABIVerifyMembershipMessage {
    fn from(value: VerifyMembershipMessage) -> Self {
        use ethabi::*;
        Self {
            prefix: value.prefix,
            path: Bytes::from(value.path),
            value: FixedBytes::from(value.value.unwrap_or_default()),
            height: EthABIHeight::from(value.height),
            state_id: value.state_id.to_vec(),
        }
    }
}

impl TryFrom<EthABIVerifyMembershipMessage> for VerifyMembershipMessage {
    type Error = Error;
    fn try_from(value: EthABIVerifyMembershipMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prefix: value.prefix,
            path: String::from_utf8(value.path)?,
            value: bytes_to_bytes32(value.value)?,
            height: value.height.into(),
            state_id: value.state_id.as_slice().try_into()?,
        })
    }
}

impl VerifyMembershipMessage {
    pub fn new(
        prefix: CommitmentPrefix,
        path: String,
        value: Option<[u8; 32]>,
        height: Height,
        state_id: StateID,
    ) -> Self {
        Self {
            prefix,
            path,
            value,
            height,
            state_id,
        }
    }
}

impl EthABIEncoder for VerifyMembershipMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIVerifyMembershipMessage>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIVerifyMembershipMessage::decode(bz).and_then(|v| v.try_into())
    }
}

use crate::encoder::{EthABIEncoder, EthABIHeight};
use crate::prelude::*;
use crate::{Error, StateID};
use alloy_sol_types::{private::B256, sol, SolValue};
use core::fmt::Display;
use lcp_types::Height;
use serde::{Deserialize, Serialize};

pub type CommitmentPrefix = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyMembershipProxyMessage {
    pub prefix: CommitmentPrefix,
    pub path: String,
    pub value: Option<[u8; 32]>,
    pub height: Height,
    pub state_id: StateID,
}

impl Display for VerifyMembershipProxyMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "VerifyMembership(prefix: {:?}, path: {}, value: {}, height: {}, state_id: {})",
            self.prefix,
            self.path,
            self.value.map_or("None".to_string(), hex::encode),
            self.height,
            self.state_id,
        )
    }
}

sol! {
    struct EthABIVerifyMembershipProxyMessage {
        bytes prefix;
        bytes path;
        bytes32 value;
        EthABIHeight height;
        bytes32 state_id;
    }
}

impl From<VerifyMembershipProxyMessage> for EthABIVerifyMembershipProxyMessage {
    fn from(msg: VerifyMembershipProxyMessage) -> Self {
        Self {
            prefix: msg.prefix.into(),
            path: msg.path.into_bytes().into(),
            value: B256::from_slice(msg.value.unwrap_or_default().as_slice()),
            height: EthABIHeight::from(msg.height),
            state_id: B256::from_slice(&msg.state_id.to_vec()),
        }
    }
}

impl TryFrom<EthABIVerifyMembershipProxyMessage> for VerifyMembershipProxyMessage {
    type Error = Error;
    fn try_from(msg: EthABIVerifyMembershipProxyMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            prefix: msg.prefix.into(),
            path: String::from_utf8(msg.path.to_vec())?,
            value: (!msg.value.is_zero()).then_some(msg.value.0),
            height: msg.height.into(),
            state_id: msg.state_id.as_slice().try_into()?,
        })
    }
}

impl VerifyMembershipProxyMessage {
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

    pub fn validate(&self) -> Result<(), Error> {
        if self.path.is_empty() {
            return Err(Error::empty_path());
        }
        if self.height.is_zero() {
            return Err(Error::zero_height());
        }
        if self.state_id.is_zero() {
            return Err(Error::zero_state_id());
        }
        Ok(())
    }
}

impl EthABIEncoder for VerifyMembershipProxyMessage {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIVerifyMembershipProxyMessage>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIVerifyMembershipProxyMessage::abi_decode(bz, true)?.try_into()
    }
}

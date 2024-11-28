use crate::{encoder::EthABIEncoder, prelude::*, Error, ProxyMessage};
use alloy_sol_types::{sol, SolValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CommitmentProof {
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
}

impl CommitmentProof {
    pub fn new(message: Vec<u8>, signature: Vec<u8>) -> Self {
        Self { message, signature }
    }

    pub fn new_with_no_signature(message: Vec<u8>) -> Self {
        Self {
            message,
            signature: Default::default(),
        }
    }

    pub fn is_proven(&self) -> bool {
        !self.signature.is_empty()
    }

    pub fn message(&self) -> Result<ProxyMessage, Error> {
        ProxyMessage::from_bytes(&self.message)
    }
}

impl EthABIEncoder for CommitmentProof {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABICommitmentProof>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        Ok(EthABICommitmentProof::abi_decode(bz, true)?.into())
    }
}

sol! {
    struct EthABICommitmentProof {
        bytes message;
        bytes signature;
    }
}

impl From<EthABICommitmentProof> for CommitmentProof {
    fn from(value: EthABICommitmentProof) -> Self {
        Self {
            message: value.message.into(),
            signature: value.signature.into(),
        }
    }
}

impl From<CommitmentProof> for EthABICommitmentProof {
    fn from(value: CommitmentProof) -> Self {
        Self {
            message: value.message.into(),
            signature: value.signature.into(),
        }
    }
}

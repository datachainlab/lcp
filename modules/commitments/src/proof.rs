use crate::{encoder::EthABIEncoder, prelude::*, Error, Message};
use crypto::Address;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommitmentProof {
    pub message: Vec<u8>,
    pub signer: Address,
    pub signature: Vec<u8>,
}

impl CommitmentProof {
    pub fn new(message: Vec<u8>, signer: Address, signature: Vec<u8>) -> Self {
        Self {
            message,
            signer,
            signature,
        }
    }

    pub fn new_with_no_signature(message: Vec<u8>) -> Self {
        Self {
            message,
            signer: Default::default(),
            signature: Default::default(),
        }
    }

    pub fn is_proven(&self) -> bool {
        !self.signature.is_empty()
    }

    pub fn message(&self) -> Result<Message, Error> {
        Message::from_bytes(&self.message)
    }
}

impl EthABIEncoder for CommitmentProof {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABICommitmentProof>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABICommitmentProof::decode(bz).map(Into::into)
    }
}

pub(crate) struct EthABICommitmentProof {
    pub message: ethabi::Bytes,
    pub signer: ethabi::Address,
    pub signature: ethabi::Bytes,
}

impl From<EthABICommitmentProof> for CommitmentProof {
    fn from(value: EthABICommitmentProof) -> Self {
        Self {
            message: value.message,
            signer: Address(value.signer.0),
            signature: value.signature,
        }
    }
}

impl EthABICommitmentProof {
    pub fn encode(self) -> Vec<u8> {
        ethabi::encode(&[ethabi::Token::Tuple(vec![
            ethabi::Token::Bytes(self.message),
            ethabi::Token::Address(self.signer),
            ethabi::Token::Bytes(self.signature),
        ])])
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let tuple = ethabi::decode(
            &[ethabi::ParamType::Tuple(vec![
                ethabi::ParamType::Bytes,
                ethabi::ParamType::Address,
                ethabi::ParamType::Bytes,
            ])],
            bytes,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 3
        assert!(tuple.len() == 3);
        let mut values = tuple.into_iter();
        Ok(Self {
            message: values.next().unwrap().into_bytes().unwrap(),
            signer: values.next().unwrap().into_address().unwrap(),
            signature: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

impl From<CommitmentProof> for EthABICommitmentProof {
    fn from(value: CommitmentProof) -> Self {
        use ethabi::*;
        Self {
            message: value.message,
            signer: Address::from(value.signer.0),
            signature: value.signature,
        }
    }
}

use crate::{commitment::EthABIEncoder, prelude::*, Commitment, Error};
use crypto::Address;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CommitmentProof {
    pub commitment_bytes: Vec<u8>,
    pub signer: Address,
    pub signature: Vec<u8>,
}

impl CommitmentProof {
    pub fn new(commitment_bytes: Vec<u8>, signer: Address, signature: Vec<u8>) -> Self {
        Self {
            commitment_bytes,
            signer,
            signature,
        }
    }

    pub fn new_with_no_signature(commitment_bytes: Vec<u8>) -> Self {
        Self {
            commitment_bytes,
            ..Default::default()
        }
    }

    pub fn commitment(&self) -> Result<Commitment, Error> {
        Commitment::from_commitment_bytes(&self.commitment_bytes)
    }

    pub fn is_proven(&self) -> bool {
        !self.signature.is_empty()
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
    pub commitment_bytes: ethabi::Bytes,
    pub signer: ethabi::Address,
    pub signature: ethabi::Bytes,
}

impl From<EthABICommitmentProof> for CommitmentProof {
    fn from(value: EthABICommitmentProof) -> Self {
        Self {
            commitment_bytes: value.commitment_bytes,
            signer: Address(value.signer.0),
            signature: value.signature,
        }
    }
}

impl EthABICommitmentProof {
    pub fn encode(self) -> Vec<u8> {
        ethabi::encode(&[ethabi::Token::Tuple(vec![
            ethabi::Token::Bytes(self.commitment_bytes),
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
            commitment_bytes: values.next().unwrap().into_bytes().unwrap(),
            signer: values.next().unwrap().into_address().unwrap(),
            signature: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

impl From<CommitmentProof> for EthABICommitmentProof {
    fn from(value: CommitmentProof) -> Self {
        use ethabi::*;
        Self {
            commitment_bytes: value.commitment_bytes,
            signer: Address::from(value.signer.0),
            signature: value.signature,
        }
    }
}

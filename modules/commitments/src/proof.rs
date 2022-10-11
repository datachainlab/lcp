use crate::prelude::*;
use crate::{commitment::UpdateClientCommitment, errors::Error, StateCommitment};
use ibc::core::ics23_commitment::commitment::CommitmentProofBytes;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateClientCommitmentProof {
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl UpdateClientCommitmentProof {
    pub fn new(commitment_bytes: Vec<u8>, signer: Vec<u8>, signature: Vec<u8>) -> Self {
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

    pub fn commitment(&self) -> UpdateClientCommitment {
        UpdateClientCommitment::from_bytes(&self.commitment_bytes).unwrap()
    }

    pub fn is_proven(&self) -> bool {
        !self.signature.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct StateCommitmentProof {
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl StateCommitmentProof {
    pub fn commitment(&self) -> StateCommitment {
        StateCommitment::from_bytes(&self.commitment_bytes).unwrap()
    }
}

impl TryFrom<CommitmentProofBytes> for StateCommitmentProof {
    type Error = Error;

    fn try_from(value: CommitmentProofBytes) -> Result<Self, Self::Error> {
        let proof = Into::<Vec<u8>>::into(value);
        let r = rlp::Rlp::new(&proof);
        Ok(Self {
            commitment_bytes: r.at(0)?.as_val::<Vec<u8>>()?,
            signer: r.at(1)?.as_val::<Vec<u8>>()?,
            signature: r.at(2)?.as_val::<Vec<u8>>()?,
        })
    }
}

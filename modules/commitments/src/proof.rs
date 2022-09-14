#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    commitment::UpdateClientCommitment, errors::CommitmentError as Error, StateCommitment,
};
use ibc::core::ics23_commitment::commitment::CommitmentProofBytes;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateClientCommitmentProof {
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl UpdateClientCommitmentProof {
    pub fn commitment(&self) -> UpdateClientCommitment {
        UpdateClientCommitment::from_bytes(&self.commitment_bytes).unwrap()
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

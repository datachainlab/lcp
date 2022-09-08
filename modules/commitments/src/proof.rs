#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    commitment::UpdateClientCommitment, errors::CommitmentError as Error, StateCommitment,
};
use ibc::core::ics23_commitment::commitment::CommitmentProofBytes;
use rlp_derive::{RlpDecodable, RlpEncodable};
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

#[derive(RlpEncodable, RlpDecodable, Serialize, Deserialize, Debug, Default)]
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
        Ok(rlp::decode(Into::<Vec<u8>>::into(value).as_slice()).map_err(Error::RLPDecoderError)?)
    }
}

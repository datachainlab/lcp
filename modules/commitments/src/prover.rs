use crate::errors::CommitmentError;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    StateCommitment, StateCommitmentProof, UpdateClientCommitment, UpdateClientCommitmentProof,
};
use enclave_crypto::EnclaveKey;

pub trait UpdateClientCommitmentProver {
    fn prove_update_client_commitment(
        &self,
        commitment: &UpdateClientCommitment,
    ) -> Result<UpdateClientCommitmentProof, CommitmentError>;
}

impl UpdateClientCommitmentProver for EnclaveKey {
    fn prove_update_client_commitment(
        &self,
        commitment: &UpdateClientCommitment,
    ) -> Result<UpdateClientCommitmentProof, CommitmentError> {
        let commitment_bytes = commitment.to_vec();
        let signature = self
            .sign(&commitment_bytes)
            .map_err(CommitmentError::CryptoError)?;
        Ok(UpdateClientCommitmentProof {
            commitment_bytes,
            signer: self.get_pubkey().get_address().to_vec(),
            signature,
        })
    }
}

pub trait StateCommitmentProver {
    fn prove_state_commitment(
        &self,
        commitment: &StateCommitment,
    ) -> Result<StateCommitmentProof, CommitmentError>;
}

impl StateCommitmentProver for EnclaveKey {
    fn prove_state_commitment(
        &self,
        commitment: &StateCommitment,
    ) -> Result<StateCommitmentProof, CommitmentError> {
        let commitment_bytes = commitment.to_vec();
        let signature = self
            .sign(&commitment_bytes)
            .map_err(CommitmentError::CryptoError)?;
        Ok(StateCommitmentProof {
            commitment_bytes,
            signer: self.get_pubkey().get_address().to_vec(),
            signature,
        })
    }
}

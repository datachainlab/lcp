use crate::errors::ProverError;
use crate::prelude::*;
use crate::{
    StateCommitment, StateCommitmentProof, UpdateClientCommitment, UpdateClientCommitmentProof,
};
use crypto::{Address, Signer};

pub fn prove_update_client_commitment(
    signer: &dyn Signer,
    signer_address: Address,
    commitment: UpdateClientCommitment,
) -> Result<UpdateClientCommitmentProof, ProverError> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer
        .sign(&commitment_bytes)
        .map_err(ProverError::crypto)?;
    Ok(UpdateClientCommitmentProof::new(
        commitment_bytes,
        signer_address.0.to_vec(),
        signature,
    ))
}

pub fn prove_state_commitment(
    signer: &dyn Signer,
    signer_address: Address,
    commitment: StateCommitment,
) -> Result<StateCommitmentProof, ProverError> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer
        .sign(&commitment_bytes)
        .map_err(ProverError::crypto)?;
    Ok(StateCommitmentProof::new(
        commitment_bytes,
        signer_address.0.to_vec(),
        signature,
    ))
}

use crate::errors::CommitmentError;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    StateCommitment, StateCommitmentProof, UpdateClientCommitment, UpdateClientCommitmentProof,
};
use store::{CommitSigner, CommitVerifier};

pub fn prove_update_client_commitment(
    signer: &dyn CommitSigner,
    commitment: &UpdateClientCommitment,
) -> Result<UpdateClientCommitmentProof, CommitmentError> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer
        .sign(&commitment_bytes)
        // .map_err(CommitmentError::CryptoError)?;
        .unwrap();
    let mut signer_address = Default::default();
    signer.use_verifier(&mut |verifier: &dyn CommitVerifier| {
        signer_address = verifier.get_address();
    });
    Ok(UpdateClientCommitmentProof {
        commitment_bytes,
        signer: signer_address,
        signature,
    })
}

pub fn prove_state_commitment(
    signer: &dyn CommitSigner,
    commitment: &StateCommitment,
) -> Result<StateCommitmentProof, CommitmentError> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer
        .sign(&commitment_bytes)
        // .map_err(CommitmentError::CryptoError)?;
        .unwrap();
    let mut signer_address = Default::default();
    signer.use_verifier(&mut |verifier: &dyn CommitVerifier| {
        signer_address = verifier.get_address();
    });
    Ok(StateCommitmentProof {
        commitment_bytes,
        signer: signer_address,
        signature,
    })
}

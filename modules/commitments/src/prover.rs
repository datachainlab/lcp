use crate::errors::Error;
use crate::{prelude::*, Commitment, CommitmentProof};
use crypto::{Address, Signer};

pub fn prove_commitment(
    signer: &dyn Signer,
    signer_address: Address,
    commitment: Commitment,
) -> Result<CommitmentProof, Error> {
    let commitment_bytes = commitment.to_commitment_bytes();
    let signature = signer.sign(&commitment_bytes).map_err(Error::crypto)?;
    Ok(CommitmentProof::new(
        commitment_bytes,
        signer_address,
        signature,
    ))
}

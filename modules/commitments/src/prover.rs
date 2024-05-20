use crate::errors::Error;
use crate::{prelude::*, CommitmentProof, ProxyMessage};
use crypto::Signer;

/// Calculate the commitment of a message and sign it
pub fn prove_commitment(
    signer: &dyn Signer,
    message: ProxyMessage,
) -> Result<CommitmentProof, Error> {
    message.validate()?;
    let message_bytes = message.to_bytes();
    let signature = signer.sign(&message_bytes).map_err(Error::crypto)?;
    Ok(CommitmentProof::new(message_bytes, signature))
}

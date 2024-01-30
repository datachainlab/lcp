use crate::errors::Error;
use crate::{prelude::*, CommitmentProof, Message};
use crypto::{Address, Signer};

/// Calculate the commitment of a message and sign it
pub fn prove_commitment(
    signer: &dyn Signer,
    signer_address: Address,
    message: Message,
) -> Result<CommitmentProof, Error> {
    message.validate()?;
    let message_bytes = message.to_bytes();
    let signature = signer.sign(&message_bytes).map_err(Error::crypto)?;
    Ok(CommitmentProof::new(
        message_bytes,
        signer_address,
        signature,
    ))
}

use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use crypto::{EnclavePublicKey, Signer, Verifier};
use ecall_commands::{AggregateMessagesInput, AggregateMessagesResult, LightClientResult};
use light_client::{
    commitments::{self, prove_commitment, Message, UpdateClientMessage},
    LightClientResolver,
};
use store::KVStore;

pub fn aggregate_messages<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: AggregateMessagesInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    if input.messages.len() < 2 {
        return Err(Error::invalid_argument(
            "messages and signatures must have at least 2 elements".into(),
        ));
    }
    if input.messages.len() != input.signatures.len() {
        return Err(Error::invalid_argument(
            "messages and signatures must have the same length".into(),
        ));
    }

    let ek = ctx.get_enclave_key();
    let pk = ek.pubkey().map_err(Error::crypto)?;

    let messages = input
        .messages
        .into_iter()
        .map(|c| Message::from_bytes(&c)?.try_into())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .zip(input.signatures.iter())
        .map(|(c, s)| -> Result<_, Error> {
            verify_commitment(&pk, &c, s)?;
            Ok(c)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let message = Message::from(commitments::aggregate_messages(messages)?);
    let proof = prove_commitment(ek, input.signer, message)?;

    Ok(LightClientResult::AggregateMessages(
        AggregateMessagesResult(proof),
    ))
}

fn verify_commitment(
    verifier: &EnclavePublicKey,
    commitment: &UpdateClientMessage,
    signature: &[u8],
) -> Result<(), Error> {
    let message_bytes = Message::UpdateClient(commitment.clone()).to_bytes();
    verifier
        .verify(&message_bytes, signature)
        .map_err(Error::crypto)?;
    Ok(())
}

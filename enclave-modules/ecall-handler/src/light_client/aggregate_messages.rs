use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use crypto::{EnclavePublicKey, Signer, Verifier};
use ecall_commands::{AggregateMessagesInput, AggregateMessagesResponse, LightClientResponse};
use light_client::{
    commitments::{self, prove_commitment, ProxyMessage, UpdateStateProxyMessage},
    HostContext, LightClientResolver,
};
use store::KVStore;

pub fn aggregate_messages<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: AggregateMessagesInput,
) -> Result<LightClientResponse, Error> {
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
        .map(|m| ProxyMessage::from_bytes(&m)?.try_into())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .zip(input.signatures.iter())
        .map(|(m, s)| -> Result<_, Error> {
            verify_message(&pk, &m, s)?;
            m.context.validate(ctx.host_timestamp())?;
            Ok(m)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let message = ProxyMessage::from(commitments::aggregate_messages(messages)?);
    let proof = prove_commitment(ek, message)?;

    Ok(LightClientResponse::AggregateMessages(
        AggregateMessagesResponse(proof),
    ))
}

fn verify_message(
    verifier: &EnclavePublicKey,
    message: &UpdateStateProxyMessage,
    signature: &[u8],
) -> Result<(), Error> {
    let message_bytes = ProxyMessage::UpdateState(message.clone()).to_bytes();
    verifier
        .verify(&message_bytes, signature)
        .map_err(Error::crypto)?;
    Ok(())
}

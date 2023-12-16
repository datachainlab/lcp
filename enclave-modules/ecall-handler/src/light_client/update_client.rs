use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use crypto::Signer;
use ecall_commands::{LightClientResult, UpdateClientInput, UpdateClientResult};
use light_client::commitments::{
    prove_commitment, CommitmentProof, EmittedState, Message, UpdateClientMessage,
};
use light_client::{ClientKeeper, LightClientResolver};
use store::KVStore;

pub fn update_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: UpdateClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    let ek = ctx.get_enclave_key();
    let res = lc.update_client(ctx, input.client_id.clone(), input.any_header.into())?;

    let message: Message = {
        let mut msg = UpdateClientMessage::try_from(res.message)?;
        if input.include_state && msg.emitted_states.is_empty() {
            msg.emitted_states = vec![EmittedState(res.height, res.new_any_client_state.clone())];
        }
        msg.into()
    };

    ctx.store_any_client_state(input.client_id.clone(), res.new_any_client_state)?;
    ctx.store_any_consensus_state(input.client_id, res.height, res.new_any_consensus_state)?;

    let proof = if res.prove {
        prove_commitment(ek, input.signer, message)?
    } else {
        CommitmentProof::new_with_no_signature(message.to_bytes())
    };
    Ok(LightClientResult::UpdateClient(UpdateClientResult(proof)))
}

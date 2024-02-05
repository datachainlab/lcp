use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use crypto::Signer;
use ecall_commands::{LightClientResponse, UpdateClientInput, UpdateClientResponse};
use light_client::commitments::{prove_commitment, CommitmentProof, EmittedState, ProxyMessage};
use light_client::{ClientKeeper, LightClientResolver, UpdateClientResult};
use store::KVStore;

pub fn update_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: UpdateClientInput,
) -> Result<LightClientResponse, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    let ek = ctx.get_enclave_key();
    match lc.update_client(ctx, input.client_id.clone(), input.any_header.into())? {
        UpdateClientResult::UpdateState(mut data) => {
            let message: ProxyMessage = {
                if input.include_state && data.message.emitted_states.is_empty() {
                    data.message.emitted_states =
                        vec![EmittedState(data.height, data.new_any_client_state.clone())];
                }
                data.message.into()
            };

            ctx.store_any_client_state(input.client_id.clone(), data.new_any_client_state)?;
            ctx.store_any_consensus_state(
                input.client_id,
                data.height,
                data.new_any_consensus_state,
            )?;

            let proof = if data.prove {
                prove_commitment(ek, input.signer, message)?
            } else {
                CommitmentProof::new_with_no_signature(message.to_bytes())
            };
            Ok(LightClientResponse::UpdateClient(UpdateClientResponse(
                proof,
            )))
        }
        UpdateClientResult::Misbehaviour(data) => {
            ctx.store_any_client_state(input.client_id, data.new_any_client_state)?;

            let proof = prove_commitment(ek, input.signer, data.message.into())?;
            Ok(LightClientResponse::UpdateClient(UpdateClientResponse(
                proof,
            )))
        }
    }
}

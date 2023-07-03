use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use commitments::{prover::prove_update_client_commitment, UpdateClientCommitmentProof};
use context::Context;
use ecall_commands::{LightClientResult, UpdateClientInput, UpdateClientResult};
use light_client::ClientKeeper;
use light_client_registry::LightClientResolver;
use store::KVStore;

pub fn update_client<R: LightClientResolver, S: KVStore>(
    ctx: &mut Context<R, S>,
    input: UpdateClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    let ek = ctx.get_enclave_key();
    let mut res = lc.update_client(ctx, input.client_id.clone(), input.any_header.into())?;
    if input.include_state && res.commitment.new_state.is_none() {
        res.commitment.new_state = Some(res.new_any_client_state.clone());
    }

    ctx.store_any_client_state(input.client_id.clone(), res.new_any_client_state)?;
    ctx.store_any_consensus_state(input.client_id, res.height, res.new_any_consensus_state)?;

    let proof = if res.prove {
        prove_update_client_commitment(ek, input.signer, res.commitment)?
    } else {
        UpdateClientCommitmentProof::new_with_no_signature(res.commitment.to_vec())
    };
    Ok(LightClientResult::UpdateClient(UpdateClientResult(proof)))
}

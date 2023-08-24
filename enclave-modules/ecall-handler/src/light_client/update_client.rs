use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use context::Context;
use crypto::Signer;
use ecall_commands::{LightClientResult, UpdateClientInput, UpdateClientResult};
use light_client::commitments::{
    prove_commitment, Commitment, CommitmentProof, UpdateClientCommitment,
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

    let commitment: Commitment = {
        let mut commitment = UpdateClientCommitment::try_from(res.commitment)?;
        if input.include_state && commitment.new_state.is_none() {
            commitment.new_state = Some(res.new_any_client_state.clone());
        }
        commitment.into()
    };

    ctx.store_any_client_state(input.client_id.clone(), res.new_any_client_state)?;
    ctx.store_any_consensus_state(input.client_id, res.height, res.new_any_consensus_state)?;

    let proof = if res.prove {
        prove_commitment(ek, input.signer, commitment)?
    } else {
        CommitmentProof::new_with_no_signature(commitment.to_commitment_bytes())
    };
    Ok(LightClientResult::UpdateClient(UpdateClientResult(proof)))
}

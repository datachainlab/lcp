use super::registry::get_light_client_by_client_id;
use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::prove_update_client_commitment;
use context::Context;
use enclave_commands::{LightClientResult, UpdateClientInput, UpdateClientResult};
use light_client::{LightClientKeeper, LightClientReader, LightClientSource};
use store::KVStore;

pub fn update_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: UpdateClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let lc = get_light_client_by_client_id::<_, L>(ctx, &input.client_id)?;

    let ek = ctx.get_enclave_key();
    let res = lc
        .update_client(ctx, input.client_id, input.any_header.into())
        .map_err(Error::LightClientError)?;

    ctx.store_any_client_state(res.client_id.clone(), res.new_any_client_state)
        .map_err(Error::ICS02Error)?;
    ctx.store_any_consensus_state(
        res.client_id.clone(),
        res.height,
        res.new_any_consensus_state,
    )
    .map_err(Error::ICS02Error)?;
    ctx.store_update_time(res.client_id.clone(), res.height, ctx.host_timestamp())
        .map_err(Error::ICS02Error)?;
    ctx.store_update_height(res.client_id, res.height, ctx.host_height())
        .map_err(Error::ICS02Error)?;

    let proof =
        prove_update_client_commitment(ek, &res.commitment).map_err(Error::CommitmentError)?;
    Ok(LightClientResult::UpdateClient(UpdateClientResult(proof)))
}

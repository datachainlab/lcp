use crate::context::{Context, LightClientKeeper};
use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::UpdateClientCommitmentProver;
use commitments::{gen_state_id_from_any, UpdateClientCommitment, UpdateClientCommitmentProof};
use enclave_commands::{InitClientInput, InitClientResult, LightClientResult};
use enclave_light_client::LightClientSource;
use enclave_store::Store;

pub fn init_client<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: InitClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestmap(input.current_timestamp);

    let lc = L::get_light_client(&input.client_type).unwrap();
    let ek = ctx.get_enclave_key();
    let res = lc
        .create_client(ctx, input.any_client_state, input.any_consensus_state)
        .map_err(Error::LightClientError)?;
    let state_id = gen_state_id_from_any(&res.any_client_state, &res.any_consensus_state)
        .map_err(Error::OtherError)?;
    let commitment = UpdateClientCommitment {
        client_id: res.client_id.clone(),
        prev_state_id: None,
        new_state_id: state_id,
        prev_height: None,
        new_height: res.height,
        timestamp: res.timestamp.nanoseconds(),
    };

    ctx.store_client_type(res.client_id.clone(), res.client_type)
        .map_err(Error::ICS02Error)?;
    ctx.store_any_client_state(res.client_id.clone(), res.any_client_state)
        .map_err(Error::ICS02Error)?;
    ctx.store_any_consensus_state(res.client_id.clone(), res.height, res.any_consensus_state)
        .map_err(Error::ICS02Error)?;
    ctx.increase_client_counter();
    ctx.store_update_time(res.client_id.clone(), res.height, res.processed_time)
        .map_err(Error::ICS02Error)?;
    ctx.store_update_height(res.client_id, res.height, res.processed_height)
        .map_err(Error::ICS02Error)?;

    let proof = ek
        .prove_update_client_commitment(&commitment)
        .map_err(Error::CommitmentError)?;
    Ok(LightClientResult::InitClient(InitClientResult(proof)))
}

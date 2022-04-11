use crate::context::{Context, LightClientKeeper, LightClientReader};
use crate::light_client::LightClientHandlerError as Error;
use commitments::{gen_state_id_from_any, ClientCommitment, ValidityProof};
use enclave_commands::{LightClientResult, UpdateClientInput, UpdateClientResult};
use enclave_light_client::LightClientSource;
use enclave_store::Store;

pub fn update_client<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: UpdateClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestmap(input.current_timestamp);

    let client_type = ctx
        .client_type(&input.client_id)
        .map_err(Error::ICS02Error)?;
    let lc = L::get_light_client(&client_type).unwrap();
    let ek = ctx.get_enclave_key();
    let res = lc
        .update_client(ctx, input.client_id, input.any_header)
        .map_err(Error::LightClientError)?;

    let prev_state_id = gen_state_id_from_any(
        &res.trusted_any_client_state,
        &res.trusted_any_consensus_state,
    )
    .map_err(Error::OtherError)?;
    let new_state_id =
        gen_state_id_from_any(&res.new_any_client_state, &res.new_any_consensus_state)
            .map_err(Error::OtherError)?;

    let commitment = ClientCommitment {
        client_id: res.client_id.clone(),
        prev_state_id: Some(prev_state_id),
        new_state_id,
        prev_height: Some(res.trusted_height),
        new_height: res.height,
        timestamp: res.timestamp.nanoseconds(),
    };
    let client_commitment_bytes = commitment.as_rlp_bytes();
    let signature = ek
        .sign(&client_commitment_bytes)
        .map_err(Error::CryptoError)?;
    let proof = ValidityProof {
        client_commitment_bytes,
        signer: ek.get_pubkey().get_address().to_vec(),
        signature,
    };

    ctx.store_any_client_state(res.client_id.clone(), res.new_any_client_state)
        .map_err(Error::ICS02Error)?;
    ctx.store_any_consensus_state(
        res.client_id.clone(),
        res.height,
        res.new_any_consensus_state,
    )
    .map_err(Error::ICS02Error)?;
    ctx.store_update_time(res.client_id.clone(), res.height, res.processed_time)
        .map_err(Error::ICS02Error)?;
    ctx.store_update_height(res.client_id, res.height, res.processed_height)
        .map_err(Error::ICS02Error)?;

    Ok(LightClientResult::UpdateClient(UpdateClientResult(proof)))
}

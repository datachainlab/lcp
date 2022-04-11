use crate::context::{Context, LightClientKeeper};
use crate::light_client::LightClientHandlerError as Error;
use enclave_light_client::client::gen_state_id_from_any;
use enclave_light_client::LightClientSource;
use enclave_store::Store;
use enclave_types::commands::{InitClientInput, InitClientResult, LightClientResult};
use enclave_types::{ClientCommitment, ValidityProof};

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
        .map_err(Error::LightClientError)?;
    let commitment = ClientCommitment {
        client_id: res.client_id.clone(),
        prev_state_id: None,
        new_state_id: state_id,
        prev_height: None,
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

    Ok(LightClientResult::InitClient(InitClientResult { proof }))
}

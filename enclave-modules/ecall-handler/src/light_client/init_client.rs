use crate::light_client::Error;
use crate::prelude::*;
use commitments::prover::prove_update_client_commitment;
use commitments::UpdateClientCommitmentProof;
use context::Context;
use core::str::FromStr;
use ecall_commands::{InitClientInput, InitClientResult, LightClientResult};
use ibc::core::ics24_host::{error::ValidationError, identifier::ClientId};
use lcp_types::Any;
use light_client::{ClientKeeper, ClientReader};
use light_client_registry::LightClientSource;
use store::KVStore;

pub fn init_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: InitClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let any_client_state: Any = input.any_client_state.into();
    let any_consensus_state: Any = input.any_consensus_state.into();
    let lc = L::get_light_client(&any_client_state.type_url).unwrap();
    let ek = ctx.get_enclave_key();
    let res = lc.create_client(ctx, any_client_state.clone(), any_consensus_state.clone())?;
    let client_id = gen_client_id(
        lc.client_type(),
        ctx.client_counter().map_err(Error::ics02)?,
    )
    .map_err(Error::ics24)?;

    ctx.store_client_type(client_id.clone(), lc.client_type())
        .map_err(Error::ics02)?;
    ctx.store_any_client_state(client_id.clone(), any_client_state)
        .map_err(Error::ics02)?;
    ctx.store_any_consensus_state(client_id.clone(), res.height, any_consensus_state)
        .map_err(Error::ics02)?;
    ctx.increase_client_counter();
    ctx.store_update_time(client_id.clone(), res.height, ctx.host_timestamp())
        .map_err(Error::ics02)?;
    ctx.store_update_height(client_id.clone(), res.height, ctx.host_height())
        .map_err(Error::ics02)?;

    let proof = if res.prove {
        prove_update_client_commitment(ek, res.commitment)?
    } else {
        UpdateClientCommitmentProof::new_with_no_signature(res.commitment.to_vec())
    };
    Ok(LightClientResult::InitClient(InitClientResult {
        client_id,
        proof,
    }))
}

fn gen_client_id(client_type: String, counter: u64) -> Result<ClientId, ValidationError> {
    ClientId::from_str(&format!("{}-{}", client_type, counter))
}

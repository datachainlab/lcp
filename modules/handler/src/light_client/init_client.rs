use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::prove_update_client_commitment;
use context::Context;
use core::str::FromStr;
use enclave_commands::{InitClientInput, InitClientResult, LightClientResult};
use ibc::core::ics24_host::{error::ValidationError, identifier::ClientId};
use lcp_types::Any;
use light_client::{ClientKeeper, ClientReader, LightClientSource};
use std::format;
use std::string::String;
use store::KVStore;

pub fn init_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: InitClientInput,
) -> Result<LightClientResult, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let any_client_state: Any = input.any_client_state.into();
    let lc = L::get_light_client(&any_client_state.type_url).unwrap();
    let ek = ctx.get_enclave_key();
    let res = lc.create_client(ctx, any_client_state, input.any_consensus_state.into())?;
    let client_id = gen_client_id(
        lc.client_type(),
        ctx.client_counter().map_err(Error::ICS02Error)?,
    )
    .map_err(Error::ICS24ValidationError)?;

    ctx.store_client_type(client_id.clone(), lc.client_type())
        .map_err(Error::ICS02Error)?;
    ctx.store_any_client_state(client_id.clone(), res.any_client_state)
        .map_err(Error::ICS02Error)?;
    ctx.store_any_consensus_state(client_id.clone(), res.height, res.any_consensus_state)
        .map_err(Error::ICS02Error)?;
    ctx.increase_client_counter();
    ctx.store_update_time(client_id.clone(), res.height, ctx.host_timestamp())
        .map_err(Error::ICS02Error)?;
    ctx.store_update_height(client_id.clone(), res.height, ctx.host_height())
        .map_err(Error::ICS02Error)?;

    let proof = prove_update_client_commitment(ek, res.commitment)?;
    Ok(LightClientResult::InitClient(InitClientResult {
        client_id,
        proof,
    }))
}

fn gen_client_id(client_type: String, counter: u64) -> Result<ClientId, ValidationError> {
    ClientId::from_str(&format!("{}-{}", client_type, counter))
}

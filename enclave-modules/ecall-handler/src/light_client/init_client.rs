use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use core::str::FromStr;
use crypto::Signer;
use ecall_commands::{InitClientInput, InitClientResponse, LightClientResponse};
use lcp_types::ClientId;
use light_client::commitments::{prove_commitment, CommitmentProof};
use light_client::{ClientKeeper, ClientReader, LightClientResolver};
use store::KVStore;

pub fn init_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: InitClientInput,
) -> Result<LightClientResponse, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let lc = match ctx.get_light_client(&input.any_client_state.type_url) {
        Some(lc) => lc,
        None => {
            return Err(Error::invalid_argument(
                input.any_client_state.type_url.clone(),
            ))
        }
    };
    let ek = ctx.get_enclave_key();
    let res = lc.create_client(
        ctx,
        input.any_client_state.clone(),
        input.any_consensus_state.clone(),
    )?;
    let client_type = lc.client_type();
    let client_id = ClientId::from_str(&input.client_id)?;
    client_id.validate(&client_type)?;

    if ctx.client_exists(&client_id) {
        return Err(Error::client_already_exists(client_id.to_string()));
    }
    ctx.store_client_type(client_id.clone(), client_type)?;
    ctx.store_any_client_state(client_id.clone(), input.any_client_state)?;
    ctx.store_any_consensus_state(client_id.clone(), res.height, input.any_consensus_state)?;

    let proof = if res.prove {
        prove_commitment(ek, res.message)?
    } else {
        CommitmentProof::new_with_no_signature(res.message.to_bytes())
    };
    Ok(LightClientResponse::InitClient(InitClientResponse {
        proof,
    }))
}

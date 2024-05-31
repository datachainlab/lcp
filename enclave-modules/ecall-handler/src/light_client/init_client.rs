use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use core::str::FromStr;
use crypto::Signer;
use ecall_commands::{InitClientInput, InitClientResponse, LightClientResponse};
use lcp_types::{Any, ClientId};
use light_client::commitments::{prove_commitment, CommitmentProof};
use light_client::{ClientKeeper, LightClientResolver};
use store::KVStore;

pub fn init_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: InitClientInput,
) -> Result<LightClientResponse, Error> {
    ctx.set_timestamp(input.current_timestamp);

    let any_client_state: Any = input.any_client_state.into();
    let any_consensus_state: Any = input.any_consensus_state.into();
    let lc = ctx.get_light_client(&any_client_state.type_url).unwrap();
    let ek = ctx.get_enclave_key();
    let res = lc.create_client(ctx, any_client_state.clone(), any_consensus_state.clone())?;
    let client_type = lc.client_type();
    let client_id = ClientId::from_str(&input.client_id)?;
    client_id.validate(&client_type)?;
    ctx.store_client_type(client_id.clone(), client_type)?;
    ctx.store_any_client_state(client_id.clone(), any_client_state)?;
    ctx.store_any_consensus_state(client_id.clone(), res.height, any_consensus_state)?;

    let proof = if res.prove {
        prove_commitment(ek, input.signer, res.message)?
    } else {
        CommitmentProof::new_with_no_signature(res.message.to_bytes())
    };
    Ok(LightClientResponse::InitClient(InitClientResponse {
        proof,
    }))
}

use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use context::Context;
use crypto::Signer;
use ecall_commands::{LightClientResponse, QueryClientInput, QueryClientResponse};
use light_client::{ClientReader, LightClientResolver};
use store::KVStore;

pub fn query_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: QueryClientInput,
) -> Result<LightClientResponse, Error> {
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    let any_client_state = ctx.client_state(&input.client_id)?;
    let any_consensus_state =
        ctx.consensus_state(&input.client_id, &lc.latest_height(ctx, &input.client_id)?)?;

    Ok(LightClientResponse::QueryClient(QueryClientResponse {
        any_client_state,
        any_consensus_state,
    }))
}

use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use context::Context;
use ecall_commands::{LightClientResult, QueryClientInput, QueryClientResult};
use light_client::ClientReader;
use light_client_registry::LightClientResolver;
use store::KVStore;

pub fn query_client<R: LightClientResolver, S: KVStore>(
    ctx: &mut Context<R, S>,
    input: QueryClientInput,
) -> Result<LightClientResult, Error> {
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    let any_client_state = ctx.client_state(&input.client_id)?;
    let any_consensus_state =
        ctx.consensus_state(&input.client_id, &lc.latest_height(ctx, &input.client_id)?)?;

    Ok(LightClientResult::QueryClient(QueryClientResult {
        any_client_state,
        any_consensus_state,
    }))
}

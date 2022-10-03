use super::registry::get_light_client_by_client_id;
use crate::light_client::LightClientHandlerError as Error;
use context::Context;
use ecall_commands::{LightClientResult, QueryClientInput, QueryClientResult};
use light_client::{ClientReader, LightClientSource};
use store::KVStore;

pub fn query_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: QueryClientInput,
) -> Result<LightClientResult, Error> {
    let lc = get_light_client_by_client_id::<_, L>(ctx, &input.client_id)?;
    let any_client_state = ctx
        .client_state(&input.client_id)
        .map_err(Error::ICS02Error)?;
    let any_consensus_state = ctx
        .consensus_state(&input.client_id, lc.latest_height(ctx, &input.client_id)?)
        .map_err(Error::ICS02Error)?;

    Ok(LightClientResult::QueryClient(QueryClientResult {
        any_client_state,
        any_consensus_state,
    }))
}

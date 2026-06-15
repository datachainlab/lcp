use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use context::Context;
use crypto::Signer;
use ecall_commands::{LightClientResponse, QueryClientInput, QueryClientResponse};
use lcp_types::store_key;
use light_client::{ClientReader, LightClientResolver};
use store::KVStore;

pub fn query_client<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: QueryClientInput,
) -> Result<LightClientResponse, Error> {
    if !ctx.client_exists(&input.client_id) {
        return Ok(LightClientResponse::QueryClient(QueryClientResponse {
            found: false,
            any_client_state: None,
            any_consensus_state: None,
            state_id: None,
            height: None,
        }));
    }
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;
    // Resolve the height to read at:
    //   - explicit `input.height` (per-height anchor lookup, per-height client_state design)
    //   - otherwise the latest tip as reported by the light client
    let height = match input.height {
        Some(h) => h,
        None => lc.latest_height(ctx, &input.client_id)?,
    };
    // For an explicit at-height request, prefer the per-height client_state
    // entry written by `store_any_client_state`; fall back to the singleton
    // when only legacy entries exist. For an unspecified request, the
    // singleton is what every existing caller observes — keep that path
    // byte-identical to pre-D behaviour.
    let any_client_state = if input.height.is_some() {
        ctx.client_state_at_height(&input.client_id, &height)?
    } else {
        ctx.client_state(&input.client_id)?
    };
    let any_consensus_state = ctx.consensus_state(&input.client_id, &height)?;
    // Best-effort state_id read; legacy entries that predate state_id tracking
    // simply return None and the caller (lcp-go drift recovery) falls back to
    // decoding the consensus_state.
    let state_id_key = store_key::state_id_bytes(input.client_id.as_str(), &height);
    let state_id = ctx.get(state_id_key.as_slice());
    Ok(LightClientResponse::QueryClient(QueryClientResponse {
        found: true,
        any_client_state: Some(any_client_state),
        any_consensus_state: Some(any_consensus_state),
        state_id,
        height: Some(height),
    }))
}

use super::types::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeUpdateClientRequest,
    SpeculativeUpdateClientResult,
};
use lcp_types::{store_key, Any, Height};
use log::warn;
use store::WriteSet;

#[derive(Debug, Clone)]
pub(crate) struct DependencyRebaseState {
    pub(crate) observed_transition: ObservedStateTransition,
    pub(crate) client_state: Option<Any>,
    pub(crate) consensus_state: Option<Any>,
}

pub(crate) fn rebase_speculative_request_in_place(
    req: &mut SpeculativeUpdateClientRequest,
    previous: &DependencyRebaseState,
) {
    // Always seed the base state from the previous result so the next unit
    // observes its predecessor's post-state and write set.
    req.base_state = ExplicitStateRef {
        prev_height: Some(previous.observed_transition.post_height),
        prev_state_id: Some(previous.observed_transition.post_state_id.clone()),
        client_state: previous.client_state.clone(),
        consensus_state: previous.consensus_state.clone(),
    };
}

pub(crate) fn build_dependency_rebase_state(
    client_id: &str,
    result: &SpeculativeUpdateClientResult,
) -> DependencyRebaseState {
    DependencyRebaseState {
        observed_transition: result.observed_transition.clone(),
        client_state: extract_client_state_from_write_set(client_id, &result.write_set),
        consensus_state: extract_consensus_state_from_write_set(
            client_id,
            result.observed_transition.post_height,
            &result.write_set,
        ),
    }
}

pub(crate) fn extract_client_state_from_write_set(
    client_id: &str,
    write_set: &WriteSet,
) -> Option<Any> {
    let key = store_key::client_state_bytes(client_id);
    decode_any_from_write_set("client_state", write_set.get(&key)?.as_ref()?)
}

pub(crate) fn extract_consensus_state_from_write_set(
    client_id: &str,
    height: Height,
    write_set: &WriteSet,
) -> Option<Any> {
    let key = store_key::consensus_state_bytes(client_id, &height);
    decode_any_from_write_set("consensus_state", write_set.get(&key)?.as_ref()?)
}

fn decode_any_from_write_set(kind: &str, value: &[u8]) -> Option<Any> {
    match bincode::serde::decode_from_slice(value, bincode::config::standard()) {
        Ok((any, _)) => Some(any),
        Err(e) => {
            warn!(
                "failed to decode {} from speculative write set: {}",
                kind, e
            );
            None
        }
    }
}

use super::types::{
    ObservedStateTransition, SpeculativeBatchFailure, SpeculativeBatchFailureKind,
    SpeculativeUpdateClientRequest, SpeculativeUpdateClientResult, MAX_SPECULATIVE_BATCH_UNITS,
    MAX_SPECULATIVE_UNIT_HEADER_BYTES,
};
use std::collections::BTreeSet;

// Validate all requests in an already materialized batch against the batch
// client ID. This is a whole-batch wrapper around per-unit admission checks and
// is mainly useful for non-streamed/test-assembled batches.
pub(crate) fn validate_linear_batch_requests(
    client_id: &str,
    units: &[SpeculativeUpdateClientRequest],
) -> core::result::Result<(), SpeculativeBatchFailure> {
    let mut seen_unit_ids = BTreeSet::new();
    for (index, unit) in units.iter().enumerate() {
        validate_next_linear_request(client_id, index, &mut seen_unit_ids, unit)?;
    }
    Ok(())
}

// Validate one request as it is admitted into a linear speculative batch. This
// check runs before speculative execution so mixed clients, duplicate unit IDs,
// oversized batches, and oversized unit headers are rejected before scheduler
// resources are spent on the unit.
pub(crate) fn validate_next_linear_request(
    client_id: &str,
    index: usize,
    seen_unit_ids: &mut BTreeSet<String>,
    req: &SpeculativeUpdateClientRequest,
) -> core::result::Result<(), SpeculativeBatchFailure> {
    if req.update_key() != client_id {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::MixedClientId,
            unit_id: Some(req.unit_id.clone()),
            detail: format!(
                "mixed client_id batch is not allowed: batch={} unit={}",
                client_id,
                req.update_key()
            ),
        });
    }
    if index >= MAX_SPECULATIVE_BATCH_UNITS {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
            unit_id: Some(req.unit_id.clone()),
            detail: format!(
                "speculative batch too large: units exceed {}",
                MAX_SPECULATIVE_BATCH_UNITS
            ),
        });
    }
    if !seen_unit_ids.insert(req.unit_id.clone()) {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::DuplicateUnitId,
            unit_id: Some(req.unit_id.clone()),
            detail: format!("duplicate unit_id in speculative batch: {}", req.unit_id),
        });
    }
    if !req.base_state.has_complete_base_state_payload() {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::BaseStateMismatch,
            unit_id: Some(req.unit_id.clone()),
            detail: format!(
                "speculative unit requires complete base_state payload: unit_id={}",
                req.unit_id
            ),
        });
    }
    let header_len = req
        .update
        .header
        .as_ref()
        .map(|header| header.value.len())
        .unwrap_or_default();
    if header_len > MAX_SPECULATIVE_UNIT_HEADER_BYTES {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
            unit_id: Some(req.unit_id.clone()),
            detail: "speculative unit header payload too large".to_string(),
        });
    }
    Ok(())
}

// Validate that the speculative execution results form a single linear chain
// in request order. This is run before stitching so a batch cannot merge write
// sets from results whose observed base/post states do not connect.
pub(crate) fn validate_linear_transitions(
    requests: &[SpeculativeUpdateClientRequest],
    results: &[SpeculativeUpdateClientResult],
) -> core::result::Result<(), SpeculativeBatchFailure> {
    let mut previous = None;
    for (req, result) in requests.iter().zip(results.iter()) {
        validate_observed_transition_follows(&req.unit_id, previous, result)?;
        previous = Some(&result.observed_transition);
    }
    Ok(())
}

// Ensure the current speculative result extends the previous unit's observed
// state transition. The first unit has no predecessor, but every following unit
// must report the previous unit's post state as its own base state before the
// batch can be stitched into one canonical write set.
//
// Binding scope: only the first unit is anchored against the local store, and
// only through stored `stateId[prev_height]`
// (`verify_expected_base_state_in_tx`). We intentionally do not require the
// latest canonical `clientState`, nor the raw `consensusState[prev_height]`
// bytes, to equal the first unit's base bytes: the on-chain/client protocol
// path may start from a historical base, and raw Any encodings are not the
// canonical identity for a light-client state. Later units are bound to their
// predecessor solely through the canonicalized state_id chain, so base fields
// erased by ELC canonicalization (for example `latest_height`) are not
// byte-compared. A divergent intermediate base from the authenticated relayer
// cannot affect the on-chain proof chain; at worst it corrupts this client's
// stitched host-store cache, which a subsequent serial update_client rewrites.
fn validate_observed_transition_follows(
    unit_id: &str,
    previous: Option<&ObservedStateTransition>,
    result: &SpeculativeUpdateClientResult,
) -> core::result::Result<(), SpeculativeBatchFailure> {
    let Some(previous) = previous else {
        return Ok(());
    };
    if result.observed_transition.prev_height != Some(previous.post_height)
        || result.observed_transition.prev_state_id.as_deref()
            != Some(previous.post_state_id.as_slice())
    {
        return Err(SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::DependencyStateMismatch,
            unit_id: Some(unit_id.to_string()),
            detail: format!(
                "unit {} base state does not match previous unit post state",
                unit_id
            ),
        });
    }
    Ok(())
}

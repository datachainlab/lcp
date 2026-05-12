use lcp_proto::lcp::service::elc::v1::{MsgUpdateClient, MsgUpdateClientResponse};
use lcp_types::{Any, Height};
use serde::{Deserialize, Serialize};
use store::WriteSet;

pub const MAX_SPECULATIVE_BATCH_UNITS: usize = 256;
pub const MAX_SPECULATIVE_BATCH_HEADER_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_SPECULATIVE_UNIT_HEADER_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitStateRef {
    pub prev_height: Option<Height>,
    pub prev_state_id: Option<Vec<u8>>,
    pub client_state: Option<Any>,
    pub consensus_state: Option<Any>,
}

impl ExplicitStateRef {
    pub(crate) fn has_complete_base_state_payload(&self) -> bool {
        self.prev_height.is_some() && self.client_state.is_some() && self.consensus_state.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedStateTransition {
    pub prev_height: Option<Height>,
    pub prev_state_id: Option<Vec<u8>>,
    pub post_height: Height,
    pub post_state_id: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeUpdateClientRequest {
    pub unit_id: String,
    pub update: MsgUpdateClient,
    pub base_state: ExplicitStateRef,
}

impl SpeculativeUpdateClientRequest {
    pub fn update_key(&self) -> String {
        self.update.client_id.clone()
    }
}

#[derive(Debug, Clone)]
pub struct SpeculativeUpdateClientResult {
    pub response: MsgUpdateClientResponse,
    pub write_set: WriteSet,
    pub base_state: ExplicitStateRef,
    pub observed_transition: ObservedStateTransition,
}

impl SpeculativeUpdateClientResult {
    #[allow(clippy::result_large_err)]
    pub fn validate_base_state(&self) -> core::result::Result<(), enclave_api::Error> {
        if self.base_state.prev_height.is_some()
            && self.base_state.prev_height != self.observed_transition.prev_height
        {
            return Err(enclave_api::Error::invalid_argument(format!(
                "base prev_height mismatch: expected={:?} observed={:?}",
                self.base_state.prev_height, self.observed_transition.prev_height
            )));
        }
        if self.base_state.prev_state_id.is_some()
            && self.base_state.prev_state_id != self.observed_transition.prev_state_id
        {
            return Err(enclave_api::Error::invalid_argument(format!(
                "base prev_state_id mismatch: expected={:?} observed={:?}",
                self.base_state.prev_state_id, self.observed_transition.prev_state_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StitchedUpdateClientResult {
    pub response: MsgUpdateClientResponse,
    pub observed_transition: ObservedStateTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeUpdateClientBatch {
    pub client_id: String,
    pub units: Vec<SpeculativeUpdateClientRequest>,
}

#[derive(Debug, Clone)]
pub struct SpeculativeUpdateClientBatchResult {
    pub client_id: String,
    pub units: Vec<SpeculativeUpdateClientResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StitchedUpdateClientBatchResult {
    pub client_id: String,
    pub units: Vec<StitchedUpdateClientResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeculativeBatchFailureKind {
    MixedClientId,
    DuplicateUnitId,
    DependencyStateMismatch,
    SpeculativeExecutionFailed,
    ResultClientMismatch,
    BatchSizeMismatch,
    BaseStateMismatch,
    StitchApplyFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeBatchFailure {
    pub kind: SpeculativeBatchFailureKind,
    pub unit_id: Option<String>,
    pub detail: String,
}

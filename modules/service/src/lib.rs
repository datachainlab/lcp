mod client_lock;
mod elc;
mod enclave;
mod service;
mod speculative;

pub use crate::service::{run_service, AppService, ElcService};
pub use crate::speculative::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeBatchFailure,
    SpeculativeBatchFailureKind, SpeculativeService, SpeculativeUpdateClientBatch,
    SpeculativeUpdateClientBatchResult, SpeculativeUpdateClientRequest,
    SpeculativeUpdateClientResult, StitchedUpdateClientBatchResult, StitchedUpdateClientResult,
    MAX_SPECULATIVE_BATCH_HEADER_BYTES, MAX_SPECULATIVE_BATCH_UNITS,
    MAX_SPECULATIVE_UNIT_HEADER_BYTES,
};

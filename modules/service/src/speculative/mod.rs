mod permit;
pub(crate) mod scheduler;
mod service;
pub(crate) mod stream;
mod types;
pub(crate) mod validation;

pub use service::SpeculativeService;
pub use types::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeBatchFailure,
    SpeculativeBatchFailureKind, SpeculativeUpdateClientBatch, SpeculativeUpdateClientBatchResult,
    SpeculativeUpdateClientRequest, SpeculativeUpdateClientResult, StitchedUpdateClientBatchResult,
    StitchedUpdateClientResult, MAX_SPECULATIVE_BATCH_HEADER_BYTES, MAX_SPECULATIVE_BATCH_UNITS,
    MAX_SPECULATIVE_UNIT_HEADER_BYTES,
};

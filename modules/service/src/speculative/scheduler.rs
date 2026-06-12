use super::service::SpeculativeService;
use super::stream::ResidentSpeculativeUpdateClientRequest;
use super::types::{
    SpeculativeBatchFailure, SpeculativeBatchFailureKind, SpeculativeUpdateClientBatchResult,
    SpeculativeUpdateClientRequest, SpeculativeUpdateClientResult,
};
use super::validation::validate_next_linear_request;
use crate::service::AppService;
use enclave_api::{EnclaveProtoAPI, SpeculativeEnclaveCommandAPI};
use log::info;
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use store::transaction::{CommitStore, TxAccessor};

pub(crate) struct StreamingSpeculativeBatchResult {
    pub(crate) requests: Vec<SpeculativeUpdateClientRequest>,
    pub(crate) results: SpeculativeUpdateClientBatchResult,
}

pub(crate) enum StreamingSpeculativeBatchInput {
    Unit(Box<ResidentSpeculativeUpdateClientRequest>),
    Complete,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}

fn speculative_request_header_len(req: &SpeculativeUpdateClientRequest) -> Option<usize> {
    Some(req.update.header.as_ref()?.value.len())
}

fn speculative_request_header_digest(
    req: &SpeculativeUpdateClientRequest,
) -> Option<(usize, String)> {
    let header = req.update.header.as_ref()?;
    Some((header.value.len(), sha256_hex(&header.value)))
}

pub(crate) fn execute_speculative_update_client_stream<E, S>(
    speculative: &SpeculativeService,
    app: &AppService<E, S>,
    client_id: String,
    inputs: Receiver<StreamingSpeculativeBatchInput>,
) -> core::result::Result<StreamingSpeculativeBatchResult, SpeculativeBatchFailure>
where
    S: CommitStore + TxAccessor + Send + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + Send + Sync + 'static,
{
    let max_parallelism = speculative.speculative_concurrency_limit();
    info!(
        "execute speculative stream: client_id={} max_parallelism={}",
        client_id, max_parallelism
    );
    let shared = Arc::new(StreamingSchedulerShared {
        state: Mutex::new(StreamingSchedulerState::new(client_id.clone())),
        ready: Condvar::new(),
        complete: Condvar::new(),
    });

    let failure = thread::scope(|scope| {
        for _ in 0..max_parallelism {
            let shared = shared.clone();
            scope.spawn(move || streaming_speculative_worker(speculative, app, shared));
        }

        let mut input_completed = false;
        for input in inputs {
            let StreamingSpeculativeBatchInput::Unit(unit) = input else {
                input_completed = true;
                break;
            };
            let mut state = shared.state.lock().unwrap();
            if state.failure.is_some() {
                break;
            }
            if let Err(e) = state.enqueue(*unit) {
                state.failure = Some(e);
                shared.ready.notify_all();
                shared.complete.notify_all();
                break;
            }
            shared.ready.notify_all();
        }

        let mut state = shared.state.lock().unwrap();
        if !input_completed {
            state
                .failure
                .get_or_insert_with(|| SpeculativeBatchFailure {
                    kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
                    unit_id: None,
                    detail: "speculative batch input stream closed before batch_end".to_string(),
                });
        }
        state.closed = true;
        shared.ready.notify_all();
        while state.failure.is_none() && state.has_unfinished_work() {
            state = shared.complete.wait(state).unwrap();
        }
        info!(
            "execute speculative stream complete: observed_max_in_flight={}",
            state.observed_max_in_flight
        );
        state.failure.clone()
    });

    if let Some(err) = failure {
        return Err(err);
    }

    let mut state = shared.state.lock().unwrap();
    let requests = (0..state.unit_count)
        .map(|index| {
            state
                .request_by_index
                .remove(&index)
                .ok_or_else(|| SpeculativeBatchFailure {
                    kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
                    unit_id: None,
                    detail: format!("unit index {} missing from executed requests", index),
                })
        })
        .collect::<core::result::Result<Vec<_>, _>>()?;
    let units = (0..state.unit_count)
        .map(|index| {
            state
                .result_by_index
                .remove(&index)
                .ok_or_else(|| SpeculativeBatchFailure {
                    kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
                    unit_id: requests.get(index).map(|req| req.unit_id.clone()),
                    detail: format!("unit index {} missing from execution results", index),
                })
        })
        .collect::<core::result::Result<Vec<_>, _>>()?;
    Ok(StreamingSpeculativeBatchResult {
        requests,
        results: SpeculativeUpdateClientBatchResult { client_id, units },
    })
}

// Shared synchronization wrapper for one streaming scheduler run.
//
// The scheduler state is protected by a single mutex so enqueue, completion, and failure transitions stay consistent across worker threads.
// `ready` wakes workers when executable units become available, while
// `complete` wakes the coordinator waiting for in-flight work to drain.
struct StreamingSchedulerShared {
    state: Mutex<StreamingSchedulerState>,
    ready: Condvar,
    complete: Condvar,
}

// Mutable state for one streaming speculative batch execution.
//
// Incoming units are assigned monotonically increasing stream indexes. Units
// are admitted only when they carry complete base-state payloads. Completed
// units store their request/result by index so the final response can be
// rebuilt in input order, even if worker threads finish out of order.
struct StreamingSchedulerState {
    client_id: String,
    ready: VecDeque<(usize, ResidentSpeculativeUpdateClientRequest)>,
    request_by_index: BTreeMap<usize, SpeculativeUpdateClientRequest>,
    result_by_index: BTreeMap<usize, SpeculativeUpdateClientResult>,
    seen_unit_ids: BTreeSet<String>,
    unit_count: usize,
    in_flight: usize,
    observed_max_in_flight: usize,
    closed: bool,
    failure: Option<SpeculativeBatchFailure>,
}

impl StreamingSchedulerState {
    fn new(client_id: String) -> Self {
        Self {
            client_id,
            ready: VecDeque::new(),
            request_by_index: BTreeMap::new(),
            result_by_index: BTreeMap::new(),
            seen_unit_ids: BTreeSet::new(),
            unit_count: 0,
            in_flight: 0,
            observed_max_in_flight: 0,
            closed: false,
            failure: None,
        }
    }

    fn has_unfinished_work(&self) -> bool {
        self.in_flight > 0 || !self.ready.is_empty()
    }

    fn enqueue(
        &mut self,
        req: ResidentSpeculativeUpdateClientRequest,
    ) -> core::result::Result<(), SpeculativeBatchFailure> {
        let index = self.unit_count;
        validate_next_linear_request(
            &self.client_id,
            index,
            &mut self.seen_unit_ids,
            req.request(),
        )?;
        self.unit_count += 1;
        self.ready.push_back((index, req));
        Ok(())
    }

    fn complete_unit(
        &mut self,
        index: usize,
        req: SpeculativeUpdateClientRequest,
        result: SpeculativeUpdateClientResult,
    ) {
        self.request_by_index.insert(index, req);
        self.result_by_index.insert(index, result);
    }
}

fn streaming_speculative_worker<E, S>(
    speculative: &SpeculativeService,
    app: &AppService<E, S>,
    shared: Arc<StreamingSchedulerShared>,
) where
    S: CommitStore + TxAccessor + Send + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + Send + Sync + 'static,
{
    loop {
        let (index, req) = {
            let mut state = shared.state.lock().unwrap();
            loop {
                if state.failure.is_some() {
                    return;
                }
                if let Some((index, req)) = state.ready.pop_front() {
                    state.in_flight += 1;
                    state.observed_max_in_flight =
                        state.observed_max_in_flight.max(state.in_flight);
                    break (index, req);
                }
                if state.closed && state.in_flight == 0 {
                    return;
                }
                state = shared.ready.wait(state).unwrap();
            }
        };

        let unit_id = req.request().unit_id.clone();
        let header_bytes = speculative_request_header_len(req.request());
        if let Some(header_bytes) = header_bytes {
            info!(
                "execute speculative update client unit: client_id={} unit_id={} header_bytes={}",
                req.request().update.client_id,
                unit_id,
                header_bytes
            );
        }
        // Dispatch the actual ECALL onto the long-lived EcallPool worker.
        // This scope thread holds the per-stream `speculative_request_permit`
        // and `in_flight` slot, then blocks on `pool.run` waiting for the pool
        // worker's result. The ECALL itself runs on the pool worker thread,
        // whose TCS binding is stable across the lifetime of the LCP service
        // process. The scope thread itself never enters the enclave and
        // therefore does not contribute to TCS occupancy.
        let pool = app.ecall_pool.clone();
        let speculative_inner = speculative.clone();
        let app_inner = app.clone();
        let req_clone = req.request().clone();
        let result = speculative
            .with_speculative_request_permit(|| {
                pool.run(move || speculative_inner.speculative_update_client(&app_inner, req_clone))
            })
            .map_err(|e| SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::SpeculativeExecutionFailed,
                unit_id: Some(unit_id),
                detail: match speculative_request_header_digest(req.request()) {
                    Some((header_bytes, header_sha256)) => format!(
                        "{}; header_bytes={} header_sha256={}",
                        e, header_bytes, header_sha256
                    ),
                    None => e.to_string(),
                },
            });

        let mut state = shared.state.lock().unwrap();
        state.in_flight -= 1;
        match result {
            Ok(result) => {
                let req = req.into_request_without_header_payload();
                state.complete_unit(index, req, result);
            }
            Err(e) => {
                state.failure.get_or_insert(e);
            }
        }
        shared.ready.notify_all();
        shared.complete.notify_all();
    }
}

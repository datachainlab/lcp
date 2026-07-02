use crate::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeUpdateClientRequest,
    StitchedUpdateClientBatchResult, StitchedUpdateClientResult, MAX_SPECULATIVE_UNIT_HEADER_BYTES,
};
#[cfg(test)]
use crate::{SpeculativeUpdateClientBatch, MAX_SPECULATIVE_BATCH_UNITS};
use lcp_proto::google::protobuf::Any;
use lcp_proto::lcp::service::elc::v1::{
    msg_speculative_update_client_batch_stream_chunk::Chunk as BatchChunk,
    ExecuteSpeculativeUpdateClientBatchResponse, ExplicitStateRef as ProtoExplicitStateRef,
    MsgSpeculativeUpdateClientBatchStreamChunk, MsgUpdateClient,
    ObservedStateTransition as ProtoObservedStateTransition,
    SpeculativeUpdateClientBatchStreamInit, SpeculativeUpdateClientUnitHeaderChunk,
    SpeculativeUpdateClientUnitInit,
    StitchedSpeculativeUpdateClientUnitResult as ProtoStitchedSpeculativeUpdateClientUnitResult,
};
use lcp_types::Height;
use log::debug;
use sha2::Digest;
use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tonic::{Status, Streaming};

pub(crate) const MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES: usize = 4 * 1024 * 1024;

// Upper bound on how long one stream may wait for header memory held by other
// streams. The budget is shared service-wide, so an unbounded wait lets two
// streams that each hold partial reservations deadlock each other (and starve
// every later stream). Timing out converts that into a retryable
// RESOURCE_EXHAUSTED error that releases the failing stream's reservations.
const SPECULATIVE_HEADER_MEMORY_RESERVE_TIMEOUT: Duration = Duration::from_secs(60);

/// Tracks the peak resident header payload bytes for one speculative batch
/// stream. Reservations are attached to decoded units and released when those
/// units are dropped after execution; this intentionally bounds in-memory
/// pressure instead of the total bytes carried by the whole stream.
#[derive(Clone, Debug)]
pub(crate) struct SpeculativeHeaderMemoryBudget {
    inner: Arc<SpeculativeHeaderMemoryBudgetInner>,
}

#[derive(Debug)]
struct SpeculativeHeaderMemoryBudgetInner {
    max_bytes: usize,
    reserve_timeout: Duration,
    state: Mutex<SpeculativeHeaderMemoryBudgetState>,
    available: Condvar,
}

#[derive(Debug, Default)]
struct SpeculativeHeaderMemoryBudgetState {
    used_bytes: usize,
}

impl SpeculativeHeaderMemoryBudget {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self::new_with_reserve_timeout(max_bytes, SPECULATIVE_HEADER_MEMORY_RESERVE_TIMEOUT)
    }

    fn new_with_reserve_timeout(max_bytes: usize, reserve_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(SpeculativeHeaderMemoryBudgetInner {
                max_bytes,
                reserve_timeout,
                state: Mutex::new(SpeculativeHeaderMemoryBudgetState::default()),
                available: Condvar::new(),
            }),
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) async fn reserve_for_chunk(
        &self,
        chunk: &MsgSpeculativeUpdateClientBatchStreamChunk,
    ) -> Result<SpeculativeHeaderMemoryReservation, Status> {
        let bytes = match chunk.chunk.as_ref() {
            Some(BatchChunk::UnitHeaderChunk(header_chunk)) => header_chunk.data.len(),
            _ => 0,
        };
        if bytes == 0 {
            return Ok(SpeculativeHeaderMemoryReservation::empty());
        }

        let budget = self.clone();
        tokio::task::spawn_blocking(move || budget.reserve_blocking(bytes))
            .await
            .map_err(|e| {
                Status::aborted(format!(
                    "speculative header memory budget waiter failed: {e}"
                ))
            })?
    }

    #[allow(clippy::result_large_err)]
    fn reserve_blocking(&self, bytes: usize) -> Result<SpeculativeHeaderMemoryReservation, Status> {
        if bytes > self.inner.max_bytes {
            return Err(Status::resource_exhausted(format!(
                "speculative resident header payload too large: bytes={} max={}",
                bytes, self.inner.max_bytes
            )));
        }

        let deadline = Instant::now() + self.inner.reserve_timeout;
        let mut state = self.inner.state.lock().unwrap();
        while state.used_bytes + bytes > self.inner.max_bytes {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Status::resource_exhausted(format!(
                    "timed out waiting for speculative header memory budget: requested_bytes={} used_bytes={} max_bytes={}",
                    bytes, state.used_bytes, self.inner.max_bytes
                )));
            }
            (state, _) = self.inner.available.wait_timeout(state, remaining).unwrap();
        }
        state.used_bytes += bytes;
        Ok(SpeculativeHeaderMemoryReservation {
            budget: Some(self.clone()),
            bytes,
        })
    }

    fn release(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let mut state = self.inner.state.lock().unwrap();
        state.used_bytes = state.used_bytes.saturating_sub(bytes);
        self.inner.available.notify_all();
    }

    #[cfg(test)]
    pub(crate) fn used_bytes(&self) -> usize {
        self.inner.state.lock().unwrap().used_bytes
    }
}

#[derive(Debug)]
pub(crate) struct SpeculativeHeaderMemoryReservation {
    budget: Option<SpeculativeHeaderMemoryBudget>,
    bytes: usize,
}

impl SpeculativeHeaderMemoryReservation {
    pub(crate) fn empty() -> Self {
        Self {
            budget: None,
            bytes: 0,
        }
    }

    fn merge(&mut self, mut other: Self) {
        if other.bytes == 0 {
            return;
        }
        if self.bytes == 0 {
            self.budget = other.budget.take();
            self.bytes = other.bytes;
            other.bytes = 0;
            return;
        }
        debug_assert!(
            match (&self.budget, &other.budget) {
                (Some(left), Some(right)) => Arc::ptr_eq(&left.inner, &right.inner),
                _ => false,
            },
            "cannot merge header memory reservations from different budgets"
        );
        self.bytes += other.bytes;
        other.bytes = 0;
    }
}

impl Drop for SpeculativeHeaderMemoryReservation {
    fn drop(&mut self) {
        if let Some(budget) = self.budget.take() {
            budget.release(self.bytes);
        }
    }
}

pub(crate) struct ResidentSpeculativeUpdateClientRequest {
    request: SpeculativeUpdateClientRequest,
    _header_memory: SpeculativeHeaderMemoryReservation,
}

impl ResidentSpeculativeUpdateClientRequest {
    fn new(
        request: SpeculativeUpdateClientRequest,
        header_memory: SpeculativeHeaderMemoryReservation,
    ) -> Self {
        Self {
            request,
            _header_memory: header_memory,
        }
    }

    pub(crate) fn request(&self) -> &SpeculativeUpdateClientRequest {
        &self.request
    }

    pub(crate) fn into_request_without_header_payload(mut self) -> SpeculativeUpdateClientRequest {
        if let Some(header) = self.request.update.header.as_mut() {
            header.value.clear();
        }
        self.request
    }

    #[cfg(test)]
    pub(crate) fn unmetered(request: SpeculativeUpdateClientRequest) -> Self {
        Self::new(request, SpeculativeHeaderMemoryReservation::empty())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}

#[derive(Debug)]
#[cfg(test)]
struct DecodedSpeculativeBatchRequest {
    client_id: String,
    units: Vec<SpeculativeUpdateClientRequest>,
}

struct OpenSpeculativeUnit {
    init: SpeculativeUpdateClientUnitInit,
    header_bytes: Vec<u8>,
    header_memory: SpeculativeHeaderMemoryReservation,
}

pub(crate) struct SpeculativeBatchStreamDecoder {
    client_id: String,
    #[cfg(test)]
    units: Vec<SpeculativeUpdateClientRequest>,
    open_unit: Option<OpenSpeculativeUnit>,
    seen_unit_ids: HashSet<String>,
    closed: bool,
}

impl SpeculativeBatchStreamDecoder {
    pub(crate) fn new(client_id: String) -> Self {
        Self {
            client_id,
            #[cfg(test)]
            units: Vec::new(),
            open_unit: None,
            seen_unit_ids: HashSet::new(),
            closed: false,
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn push_chunk(
        &mut self,
        chunk: Option<BatchChunk>,
        header_memory: SpeculativeHeaderMemoryReservation,
    ) -> Result<Option<ResidentSpeculativeUpdateClientRequest>, Status> {
        if self.closed {
            return Err(Status::invalid_argument(
                "speculative batch stream received chunk after batch_end",
            ));
        }
        match chunk {
            Some(BatchChunk::Init(_)) => Err(Status::invalid_argument(
                "Init must only appear as the first message",
            )),
            Some(BatchChunk::UnitInit(unit_init)) => {
                if self.open_unit.is_some() {
                    return Err(Status::invalid_argument(
                        "speculative unit_init received before previous unit_end",
                    ));
                }
                if self.seen_unit_ids.contains(&unit_init.unit_id) {
                    return Err(Status::invalid_argument(format!(
                        "duplicate speculative unit_id: {}",
                        unit_init.unit_id
                    )));
                }
                validate_speculative_unit_init(&unit_init)?;
                self.open_unit = Some(OpenSpeculativeUnit {
                    init: unit_init,
                    header_bytes: Vec::new(),
                    header_memory: SpeculativeHeaderMemoryReservation::empty(),
                });
                Ok(None)
            }
            Some(BatchChunk::UnitHeaderChunk(header_chunk)) => {
                append_speculative_unit_header_chunk(
                    &mut self.open_unit,
                    header_chunk,
                    header_memory,
                )?;
                Ok(None)
            }
            Some(BatchChunk::UnitEnd(unit_end)) => {
                let unit = close_speculative_unit(
                    &self.client_id,
                    self.open_unit.take(),
                    unit_end.unit_id,
                )?;
                if !self.seen_unit_ids.insert(unit.request().unit_id.clone()) {
                    return Err(Status::invalid_argument(format!(
                        "duplicate speculative unit_id: {}",
                        unit.request().unit_id
                    )));
                }
                #[cfg(test)]
                self.units.push(unit.request().clone());
                Ok(Some(unit))
            }
            Some(BatchChunk::BatchEnd(_)) => {
                if self.open_unit.is_some() {
                    return Err(Status::invalid_argument(
                        "speculative batch_end received while chunked unit is open",
                    ));
                }
                self.closed = true;
                Ok(None)
            }
            None => Err(Status::invalid_argument("received empty chunk message")),
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn finish(&self) -> Result<(), Status> {
        if self.open_unit.is_some() {
            return Err(Status::invalid_argument(
                "speculative batch stream ended while chunked unit is open",
            ));
        }
        if !self.closed {
            return Err(Status::invalid_argument(
                "speculative batch stream ended without batch_end",
            ));
        }
        Ok(())
    }
}

pub(crate) async fn decode_speculative_batch_stream_init(
    stream: &mut Streaming<MsgSpeculativeUpdateClientBatchStreamChunk>,
) -> Result<SpeculativeUpdateClientBatchStreamInit, Status> {
    match stream.message().await? {
        Some(chunk) => match chunk.chunk {
            Some(BatchChunk::Init(init)) => validate_speculative_batch_stream_init(init),
            _ => Err(Status::invalid_argument(
                "first message must be of type Init",
            )),
        },
        None => Err(Status::invalid_argument(
            "expected Init message as the first message",
        )),
    }
}

#[allow(clippy::result_large_err)]
fn validate_speculative_batch_stream_init(
    init: SpeculativeUpdateClientBatchStreamInit,
) -> Result<SpeculativeUpdateClientBatchStreamInit, Status> {
    if init.client_id.is_empty() {
        return Err(Status::invalid_argument(
            "speculative batch stream init requires client_id",
        ));
    }
    Ok(init)
}

#[allow(clippy::result_large_err)]
fn validate_speculative_unit_init(
    unit_init: &SpeculativeUpdateClientUnitInit,
) -> Result<(), Status> {
    if unit_init.unit_id.is_empty() {
        return Err(Status::invalid_argument(
            "speculative unit_init requires unit_id",
        ));
    }
    if unit_init.type_url.is_empty() {
        return Err(Status::invalid_argument(
            "speculative unit_init requires type_url",
        ));
    }
    if unit_init.base_state.is_none() {
        return Err(Status::invalid_argument(
            "speculative unit_init requires base_state",
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn append_speculative_unit_header_chunk(
    open_unit: &mut Option<OpenSpeculativeUnit>,
    header_chunk: SpeculativeUpdateClientUnitHeaderChunk,
    header_memory: SpeculativeHeaderMemoryReservation,
) -> Result<(), Status> {
    if header_chunk.data.is_empty() {
        return Err(Status::invalid_argument(
            "speculative unit_header_chunk data must not be empty",
        ));
    }
    if header_chunk.data.len() > MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES {
        return Err(Status::resource_exhausted(format!(
            "speculative unit_header_chunk too large: bytes={} max={}",
            header_chunk.data.len(),
            MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES
        )));
    }

    let Some(open) = open_unit.as_mut() else {
        return Err(Status::invalid_argument(
            "speculative unit_header_chunk received before unit_init",
        ));
    };
    if header_chunk.unit_id != open.init.unit_id {
        return Err(Status::invalid_argument(format!(
            "speculative unit_header_chunk unit_id mismatch: open={} chunk={}",
            open.init.unit_id, header_chunk.unit_id
        )));
    }

    open.header_bytes.extend(header_chunk.data);
    open.header_memory.merge(header_memory);
    validate_speculative_unit_header_payload_len(&open.init.unit_id, open.header_bytes.len())?;
    Ok(())
}

#[allow(clippy::result_large_err)]
fn validate_speculative_unit_header_payload_len(
    unit_id: &str,
    header_bytes: usize,
) -> Result<(), Status> {
    if header_bytes > MAX_SPECULATIVE_UNIT_HEADER_BYTES {
        return Err(Status::resource_exhausted(format!(
            "speculative unit header payload too large: unit_id={} bytes={} max={}",
            unit_id, header_bytes, MAX_SPECULATIVE_UNIT_HEADER_BYTES
        )));
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn close_speculative_unit(
    client_id: &str,
    open_unit: Option<OpenSpeculativeUnit>,
    unit_id: String,
) -> Result<ResidentSpeculativeUpdateClientRequest, Status> {
    let Some(open) = open_unit else {
        return Err(Status::invalid_argument(
            "speculative unit_end received before unit_init",
        ));
    };
    if unit_id != open.init.unit_id {
        return Err(Status::invalid_argument(format!(
            "speculative unit_end unit_id mismatch: open={} end={}",
            open.init.unit_id, unit_id
        )));
    }
    if open.header_bytes.is_empty() {
        return Err(Status::invalid_argument(format!(
            "speculative unit header is empty: unit_id={}",
            open.init.unit_id
        )));
    }
    debug!(
        "received speculative update client unit: client_id={} unit_id={} header_bytes={} header_sha256={}",
        client_id,
        open.init.unit_id,
        open.header_bytes.len(),
        sha256_hex(&open.header_bytes)
    );

    let request = SpeculativeUpdateClientRequest {
        unit_id: open.init.unit_id,
        update: MsgUpdateClient {
            client_id: client_id.to_string(),
            header: Some(Any {
                type_url: open.init.type_url,
                value: open.header_bytes,
            }),
            include_state: open.init.include_state,
            signer: open.init.signer,
        },
        base_state: decode_explicit_state_ref(open.init.base_state)?,
    };
    Ok(ResidentSpeculativeUpdateClientRequest::new(
        request,
        open.header_memory,
    ))
}

#[allow(clippy::result_large_err)]
#[cfg(test)]
fn decode_speculative_batch(
    request: DecodedSpeculativeBatchRequest,
) -> Result<SpeculativeUpdateClientBatch, Status> {
    validate_speculative_batch_limits(&request)?;
    Ok(SpeculativeUpdateClientBatch {
        client_id: request.client_id,
        units: request.units,
    })
}

#[allow(clippy::result_large_err)]
#[cfg(test)]
fn validate_speculative_batch_limits(
    request: &DecodedSpeculativeBatchRequest,
) -> Result<(), Status> {
    if request.units.len() > MAX_SPECULATIVE_BATCH_UNITS {
        return Err(Status::invalid_argument(format!(
            "speculative batch too large: units={} max={}",
            request.units.len(),
            MAX_SPECULATIVE_BATCH_UNITS
        )));
    }
    for unit in &request.units {
        let header_bytes = unit
            .update
            .header
            .as_ref()
            .map(|header| header.value.len())
            .unwrap_or_default();
        validate_speculative_unit_header_payload_len(&unit.unit_id, header_bytes)?;
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn decode_explicit_state_ref(
    base_state: Option<ProtoExplicitStateRef>,
) -> Result<ExplicitStateRef, Status> {
    let base_state =
        base_state.ok_or_else(|| Status::invalid_argument("missing speculative base_state"))?;
    Ok(ExplicitStateRef {
        prev_height: base_state.prev_height.map(Height::from),
        prev_state_id: if base_state.prev_state_id.is_empty() {
            None
        } else {
            Some(base_state.prev_state_id)
        },
        client_state: base_state.client_state.map(Into::into),
        consensus_state: base_state.consensus_state.map(Into::into),
    })
}

pub(crate) fn encode_stitched_batch_result(
    result: StitchedUpdateClientBatchResult,
) -> ExecuteSpeculativeUpdateClientBatchResponse {
    ExecuteSpeculativeUpdateClientBatchResponse {
        client_id: result.client_id,
        units: result
            .units
            .into_iter()
            .map(encode_stitched_unit_result)
            .collect(),
    }
}

fn encode_stitched_unit_result(
    result: StitchedUpdateClientResult,
) -> ProtoStitchedSpeculativeUpdateClientUnitResult {
    ProtoStitchedSpeculativeUpdateClientUnitResult {
        response: Some(result.response),
        observed_transition: Some(encode_observed_transition(result.observed_transition)),
    }
}

fn encode_observed_transition(transition: ObservedStateTransition) -> ProtoObservedStateTransition {
    ProtoObservedStateTransition {
        prev_height: transition.prev_height.map(Into::into),
        prev_state_id: transition.prev_state_id.unwrap_or_default(),
        post_height: Some(transition.post_height.into()),
        post_state_id: transition.post_state_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_speculative_batch, validate_speculative_batch_stream_init,
        validate_speculative_unit_header_payload_len, DecodedSpeculativeBatchRequest,
        SpeculativeBatchStreamDecoder, SpeculativeHeaderMemoryBudget,
        SpeculativeHeaderMemoryReservation, MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES,
    };
    use crate::{
        ExplicitStateRef, SpeculativeUpdateClientRequest, MAX_SPECULATIVE_BATCH_UNITS,
        MAX_SPECULATIVE_UNIT_HEADER_BYTES,
    };
    use lcp_proto::google::protobuf::Any;
    use lcp_proto::lcp::service::elc::v1::{
        msg_speculative_update_client_batch_stream_chunk::Chunk as BatchChunk,
        ExplicitStateRef as ProtoExplicitStateRef, MsgSpeculativeUpdateClientBatchStreamChunk,
        MsgUpdateClient, SpeculativeUpdateClientBatchEnd, SpeculativeUpdateClientBatchStreamInit,
        SpeculativeUpdateClientUnitEnd, SpeculativeUpdateClientUnitHeaderChunk,
        SpeculativeUpdateClientUnitInit,
    };
    use tonic::Code;

    fn make_unit(unit_id: usize, header_len: usize) -> SpeculativeUpdateClientRequest {
        SpeculativeUpdateClientRequest {
            unit_id: format!("unit-{unit_id:04}"),
            update: MsgUpdateClient {
                client_id: "client-0".to_string(),
                header: Some(Any {
                    type_url: "/test.Header".to_string(),
                    value: vec![0u8; header_len],
                }),
                include_state: false,
                signer: Vec::new(),
            },
            base_state: ExplicitStateRef {
                prev_height: None,
                prev_state_id: None,
                client_state: None,
                consensus_state: None,
            },
        }
    }

    fn make_unit_init(unit_id: &str) -> SpeculativeUpdateClientUnitInit {
        SpeculativeUpdateClientUnitInit {
            unit_id: unit_id.to_string(),
            type_url: "/test.Header".to_string(),
            include_state: false,
            signer: Vec::new(),
            base_state: Some(ProtoExplicitStateRef {
                prev_height: None,
                prev_state_id: Vec::new(),
                client_state: None,
                consensus_state: None,
            }),
        }
    }

    #[allow(clippy::result_large_err)]
    fn decode_stream_chunks(
        chunks: impl IntoIterator<Item = BatchChunk>,
    ) -> Result<DecodedSpeculativeBatchRequest, tonic::Status> {
        let mut decoder = SpeculativeBatchStreamDecoder::new("client-0".to_string());
        for chunk in chunks {
            decoder.push_chunk(Some(chunk), SpeculativeHeaderMemoryReservation::empty())?;
        }
        decoder.finish()?;
        Ok(DecodedSpeculativeBatchRequest {
            client_id: decoder.client_id,
            units: decoder.units,
        })
    }

    fn assert_invalid_argument_contains(err: tonic::Status, expected_message: &str) {
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(
            err.message().contains(expected_message),
            "unexpected error message: {}",
            err.message()
        );
    }

    fn assert_resource_exhausted_contains(err: tonic::Status, expected_message: &str) {
        assert_eq!(err.code(), Code::ResourceExhausted);
        assert!(
            err.message().contains(expected_message),
            "unexpected error message: {}",
            err.message()
        );
    }

    fn header_chunk_msg(
        unit_id: &str,
        data: Vec<u8>,
    ) -> MsgSpeculativeUpdateClientBatchStreamChunk {
        MsgSpeculativeUpdateClientBatchStreamChunk {
            chunk: Some(BatchChunk::UnitHeaderChunk(
                SpeculativeUpdateClientUnitHeaderChunk {
                    unit_id: unit_id.to_string(),
                    data,
                },
            )),
        }
    }

    #[test]
    fn header_memory_reservation_is_held_by_decoded_unit_until_drop() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let budget = SpeculativeHeaderMemoryBudget::new(10);
        let mut decoder = SpeculativeBatchStreamDecoder::new("client-0".to_string());

        decoder
            .push_chunk(
                Some(BatchChunk::UnitInit(make_unit_init("unit-0000"))),
                SpeculativeHeaderMemoryReservation::empty(),
            )
            .expect("unit init");
        let chunk_msg = header_chunk_msg("unit-0000", b"abc".to_vec());
        let header_memory = runtime
            .block_on(budget.reserve_for_chunk(&chunk_msg))
            .expect("header memory");
        assert_eq!(budget.used_bytes(), 3);
        decoder
            .push_chunk(chunk_msg.chunk, header_memory)
            .expect("header chunk");
        assert_eq!(budget.used_bytes(), 3);

        let unit = decoder
            .push_chunk(
                Some(BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                    unit_id: "unit-0000".to_string(),
                })),
                SpeculativeHeaderMemoryReservation::empty(),
            )
            .expect("unit end")
            .expect("decoded unit");
        assert_eq!(budget.used_bytes(), 3);
        drop(unit);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn header_memory_reservation_wait_times_out_instead_of_deadlocking() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let budget = super::SpeculativeHeaderMemoryBudget::new_with_reserve_timeout(
            10,
            std::time::Duration::from_millis(50),
        );
        let held = runtime
            .block_on(budget.reserve_for_chunk(&header_chunk_msg("unit-0000", vec![0u8; 8])))
            .expect("first reservation");

        let err = runtime
            .block_on(budget.reserve_for_chunk(&header_chunk_msg("unit-0001", vec![0u8; 8])))
            .expect_err("reservation exceeding the budget must time out");
        assert_resource_exhausted_contains(
            err,
            "timed out waiting for speculative header memory budget",
        );

        // Releasing the held reservation makes the budget usable again.
        drop(held);
        runtime
            .block_on(budget.reserve_for_chunk(&header_chunk_msg("unit-0002", vec![0u8; 8])))
            .expect("reservation after release");
    }

    #[test]
    fn decode_speculative_batch_rejects_too_many_units() {
        let request = DecodedSpeculativeBatchRequest {
            client_id: "client-0".to_string(),
            units: (0..=MAX_SPECULATIVE_BATCH_UNITS)
                .map(|i| make_unit(i, 1))
                .collect(),
        };
        let err = decode_speculative_batch(request).unwrap_err();
        assert!(err.message().contains("speculative batch too large"));
    }

    #[test]
    fn validate_speculative_unit_header_payload_len_rejects_excessive_payload() {
        let err = validate_speculative_unit_header_payload_len(
            "unit-0000",
            MAX_SPECULATIVE_UNIT_HEADER_BYTES + 1,
        )
        .unwrap_err();
        assert_resource_exhausted_contains(err, "speculative unit header payload too large");
    }

    #[test]
    fn validate_speculative_batch_stream_init_rejects_empty_client_id() {
        let err = validate_speculative_batch_stream_init(SpeculativeUpdateClientBatchStreamInit {
            client_id: String::new(),
        })
        .unwrap_err();
        assert_invalid_argument_contains(err, "requires client_id");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_decodes_units() {
        let request = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: b"abc".to_vec(),
            }),
            BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                unit_id: "unit-0000".to_string(),
            }),
            BatchChunk::BatchEnd(SpeculativeUpdateClientBatchEnd {}),
        ])
        .unwrap();

        assert_eq!(request.units.len(), 1);
        assert_eq!(request.units[0].unit_id, "unit-0000");
        assert_eq!(
            request.units[0].update.header.as_ref().unwrap().value,
            b"abc"
        );
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_second_init() {
        let err =
            decode_stream_chunks([BatchChunk::Init(SpeculativeUpdateClientBatchStreamInit {
                client_id: "client-0".to_string(),
            })])
            .unwrap_err();
        assert_invalid_argument_contains(err, "Init must only appear");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_nested_unit_init() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitInit(make_unit_init("unit-0001")),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "unit_init received before previous unit_end");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_header_chunk_before_unit_init() {
        let err = decode_stream_chunks([BatchChunk::UnitHeaderChunk(
            SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: b"abc".to_vec(),
            },
        )])
        .unwrap_err();
        assert_invalid_argument_contains(err, "unit_header_chunk received before unit_init");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_chunk_unit_id_mismatch() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0001".to_string(),
                data: b"abc".to_vec(),
            }),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "unit_header_chunk unit_id mismatch");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_end_unit_id_mismatch() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: b"abc".to_vec(),
            }),
            BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                unit_id: "unit-0001".to_string(),
            }),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "unit_end unit_id mismatch");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_oversized_chunk() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: vec![0u8; MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES + 1],
            }),
        ])
        .unwrap_err();
        assert_resource_exhausted_contains(err, "unit_header_chunk too large");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_eof_with_open_unit() {
        let err =
            decode_stream_chunks([BatchChunk::UnitInit(make_unit_init("unit-0000"))]).unwrap_err();
        assert_invalid_argument_contains(err, "stream ended while chunked unit is open");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_eof_without_batch_end() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: b"abc".to_vec(),
            }),
            BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                unit_id: "unit-0000".to_string(),
            }),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "stream ended without batch_end");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_empty_header() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                unit_id: "unit-0000".to_string(),
            }),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "speculative unit header is empty");
    }

    #[test]
    fn decode_speculative_batch_stream_chunks_rejects_duplicate_unit_id() {
        let err = decode_stream_chunks([
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
            BatchChunk::UnitHeaderChunk(SpeculativeUpdateClientUnitHeaderChunk {
                unit_id: "unit-0000".to_string(),
                data: b"abc".to_vec(),
            }),
            BatchChunk::UnitEnd(SpeculativeUpdateClientUnitEnd {
                unit_id: "unit-0000".to_string(),
            }),
            BatchChunk::UnitInit(make_unit_init("unit-0000")),
        ])
        .unwrap_err();
        assert_invalid_argument_contains(err, "duplicate speculative unit_id");
    }
}

use crate::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeUpdateClientRequest,
    StitchedUpdateClientBatchResult, StitchedUpdateClientResult,
    MAX_SPECULATIVE_BATCH_HEADER_BYTES, MAX_SPECULATIVE_UNIT_HEADER_BYTES,
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
use log::info;
use sha2::Digest;
use std::collections::HashSet;
use tonic::{Status, Streaming};

pub(crate) const MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES: usize = 4 * 1024 * 1024;

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
}

pub(crate) struct SpeculativeBatchStreamDecoder {
    client_id: String,
    #[cfg(test)]
    units: Vec<SpeculativeUpdateClientRequest>,
    open_unit: Option<OpenSpeculativeUnit>,
    seen_unit_ids: HashSet<String>,
    closed: bool,
    total_header_bytes: usize,
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
            total_header_bytes: 0,
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn push_chunk(
        &mut self,
        chunk: Option<BatchChunk>,
    ) -> Result<Option<SpeculativeUpdateClientRequest>, Status> {
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
                });
                Ok(None)
            }
            Some(BatchChunk::UnitHeaderChunk(header_chunk)) => {
                append_speculative_unit_header_chunk(
                    &mut self.open_unit,
                    header_chunk,
                    &mut self.total_header_bytes,
                )?;
                Ok(None)
            }
            Some(BatchChunk::UnitEnd(unit_end)) => {
                let unit = close_speculative_unit(
                    &self.client_id,
                    self.open_unit.take(),
                    unit_end.unit_id,
                )?;
                if !self.seen_unit_ids.insert(unit.unit_id.clone()) {
                    return Err(Status::invalid_argument(format!(
                        "duplicate speculative unit_id: {}",
                        unit.unit_id
                    )));
                }
                #[cfg(test)]
                self.units.push(unit.clone());
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
            Some(BatchChunk::Init(init)) => Ok(init),
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
    total_header_bytes: &mut usize,
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

    let chunk_len = header_chunk.data.len();
    open.header_bytes.extend(header_chunk.data);
    *total_header_bytes += chunk_len;
    validate_speculative_unit_header_payload_len(&open.init.unit_id, open.header_bytes.len())?;
    if *total_header_bytes > MAX_SPECULATIVE_BATCH_HEADER_BYTES {
        return Err(Status::resource_exhausted(format!(
            "speculative batch header payload too large: bytes={} max={}",
            *total_header_bytes, MAX_SPECULATIVE_BATCH_HEADER_BYTES
        )));
    }
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
) -> Result<SpeculativeUpdateClientRequest, Status> {
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
    info!(
        "received speculative update client unit: client_id={} unit_id={} header_bytes={} header_sha256={}",
        client_id,
        open.init.unit_id,
        open.header_bytes.len(),
        sha256_hex(&open.header_bytes)
    );

    Ok(SpeculativeUpdateClientRequest {
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
    })
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
        decode_speculative_batch, validate_speculative_unit_header_payload_len,
        DecodedSpeculativeBatchRequest, SpeculativeBatchStreamDecoder,
        MAX_SPECULATIVE_BATCH_HEADER_CHUNK_BYTES,
    };
    use crate::{
        ExplicitStateRef, SpeculativeUpdateClientRequest, MAX_SPECULATIVE_BATCH_UNITS,
        MAX_SPECULATIVE_UNIT_HEADER_BYTES,
    };
    use lcp_proto::google::protobuf::Any;
    use lcp_proto::lcp::service::elc::v1::{
        msg_speculative_update_client_batch_stream_chunk::Chunk as BatchChunk,
        ExplicitStateRef as ProtoExplicitStateRef, MsgUpdateClient,
        SpeculativeUpdateClientBatchEnd, SpeculativeUpdateClientBatchStreamInit,
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
            decoder.push_chunk(Some(chunk))?;
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

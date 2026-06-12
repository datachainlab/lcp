use crate::service::{AppService, ElcService};
use crate::speculative::scheduler::StreamingSpeculativeBatchInput;
use crate::speculative::stream::{
    decode_speculative_batch_stream_init, encode_stitched_batch_result,
    SpeculativeBatchStreamDecoder,
};
use enclave_api::{EnclaveProtoAPI, SpeculativeEnclaveCommandAPI};
use lcp_proto::google::protobuf::Any;
use lcp_proto::lcp::service::elc::v1::msg_update_client_stream_chunk::Chunk;
use lcp_proto::lcp::service::elc::v1::{
    msg_server::Msg, query_server::Query, ExecuteSpeculativeUpdateClientBatchResponse,
    MsgAggregateMessages, MsgAggregateMessagesResponse, MsgCreateClient, MsgCreateClientResponse,
    MsgSpeculativeUpdateClientBatchStreamChunk, MsgUpdateClient, MsgUpdateClientResponse,
    MsgUpdateClientStreamChunk, MsgVerifyMembership, MsgVerifyMembershipResponse,
    MsgVerifyNonMembership, MsgVerifyNonMembershipResponse, QueryClientRequest,
    QueryClientResponse,
};
use log::{debug, warn};
use std::sync::mpsc;
use std::time::Duration;
use store::transaction::{CommitStore, TxAccessor};
use tokio::time::timeout;
use tonic::{Request, Response, Status, Streaming};

const SPECULATIVE_BATCH_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[tonic::async_trait]
impl<E, S> Msg for ElcService<E, S>
where
    S: CommitStore + TxAccessor + Send + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + Send + Sync + 'static,
{
    async fn create_client(
        &self,
        request: Request<MsgCreateClient>,
    ) -> Result<Response<MsgCreateClientResponse>, Status> {
        let inner = request.into_inner();
        let app = self.app.clone();
        let result = tokio::task::spawn_blocking(move || {
            app.ecall_pool
                .run(move || app.enclave.proto_create_client(inner))
        })
        .await
        .map_err(|e| Status::aborted(format!("create client worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn update_client(
        &self,
        request: Request<MsgUpdateClient>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        let msg = request.into_inner();
        let client_id = msg.client_id.clone();
        let service = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let pool = service.app.ecall_pool.clone();
            let enclave = service.app.enclave.clone();
            service.with_client_update_serialized(&client_id, move || {
                // The blocking-pool thread holds the per-client lock; the
                // actual ECALL runs on an EcallPool worker so cumulative
                // TCS bindings stay bounded.
                pool.run(move || enclave.proto_update_client(msg))
            })
        })
        .await
        .map_err(|e| Status::aborted(format!("update client worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn update_client_stream(
        &self,
        request: Request<Streaming<MsgUpdateClientStreamChunk>>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        let mut stream = request.into_inner();

        // read the first message (must be Init)
        let init = match stream.message().await? {
            Some(chunk) => match chunk.chunk {
                Some(Chunk::Init(init)) => init,
                _ => {
                    return Err(Status::invalid_argument(
                        "first message must be of type Init",
                    ))
                }
            },
            None => {
                return Err(Status::invalid_argument(
                    "expected Init message as the first message",
                ))
            }
        };

        // accumulate header chunks
        let mut header_bytes = Vec::new();

        while let Some(chunk_msg) = stream.message().await? {
            match chunk_msg.chunk {
                Some(Chunk::HeaderChunk(header_chunk)) => {
                    header_bytes.extend(header_chunk.data);
                }
                Some(Chunk::Init(_)) => {
                    return Err(Status::invalid_argument(
                        "Init must only appear as the first message",
                    ));
                }
                None => {
                    return Err(Status::invalid_argument("received empty chunk message"));
                }
            }
        }

        if header_bytes.is_empty() {
            return Err(Status::invalid_argument("no header data received"));
        }

        // create MsgUpdateClient from Init and collected header data
        let msg = MsgUpdateClient {
            client_id: init.client_id,
            include_state: init.include_state,
            signer: init.signer,
            header: Some(Any {
                type_url: init.type_url,
                value: header_bytes,
            }),
        };

        let client_id = msg.client_id.clone();
        let service = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let pool = service.app.ecall_pool.clone();
            let enclave = service.app.enclave.clone();
            service.with_client_update_serialized(&client_id, move || {
                pool.run(move || enclave.proto_update_client(msg))
            })
        })
        .await
        .map_err(|e| Status::aborted(format!("update client stream worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn speculative_update_client_batch_stream(
        &self,
        request: Request<Streaming<MsgSpeculativeUpdateClientBatchStreamChunk>>,
    ) -> Result<Response<ExecuteSpeculativeUpdateClientBatchResponse>, Status> {
        let mut stream = request.into_inner();
        let init = decode_speculative_batch_stream_init(&mut stream).await?;
        let client_id = init.client_id;
        // This channel is intentionally unbounded: resident header bytes are
        // bounded by the service-global `SpeculativeHeaderMemoryBudget`, which
        // is the actual backpressure mechanism for large speculative batch
        // inputs across concurrent streams.
        let (tx, rx) = mpsc::channel();
        let app = self.app.clone();
        let speculative = self.speculative.clone();
        let scheduler_client_id = client_id.clone();
        let service = self.clone();
        let scheduler = tokio::task::spawn_blocking(move || {
            let (batch, results) = speculative.execute_speculative_update_client_stream_batch(
                &app,
                scheduler_client_id.clone(),
                rx,
            )?;
            service.with_client_update_serialized(&scheduler_client_id, || {
                speculative.stitch_executed_speculative_update_client_stream(&app, batch, results)
            })
        });
        let mut decoder = SpeculativeBatchStreamDecoder::new(client_id.clone());
        let header_memory_budget = self.speculative.header_memory_budget();
        let mut units = 0usize;

        loop {
            let chunk_msg = match timeout(SPECULATIVE_BATCH_STREAM_IDLE_TIMEOUT, stream.message())
                .await
            {
                Ok(result) => result?,
                Err(_) => {
                    warn!(
                        "speculative update client batch stream idle timeout: client_id={} timeout_secs={}",
                        client_id,
                        SPECULATIVE_BATCH_STREAM_IDLE_TIMEOUT.as_secs()
                    );
                    drop(tx);
                    let _ = scheduler.await;
                    return Err(Status::deadline_exceeded(format!(
                        "speculative update client batch stream idle timeout after {} seconds",
                        SPECULATIVE_BATCH_STREAM_IDLE_TIMEOUT.as_secs()
                    )));
                }
            };
            let Some(chunk_msg) = chunk_msg else {
                break;
            };
            let header_memory = header_memory_budget.reserve_for_chunk(&chunk_msg).await?;
            if let Some(unit) = decoder.push_chunk(chunk_msg.chunk, header_memory)? {
                units += 1;
                if tx
                    .send(StreamingSpeculativeBatchInput::Unit(Box::new(unit)))
                    .is_err()
                {
                    let result = scheduler.await.map_err(|e| {
                        Status::aborted(format!("speculative batch worker failed: {e}"))
                    })?;
                    return match result {
                        Ok(_) => Err(Status::aborted(
                            "speculative batch scheduler stopped before stream ended",
                        )),
                        Err(e) => Err(Status::aborted(format!("{:?}: {}", e.kind, e.detail))),
                    };
                }
            }
        }
        decoder.finish()?;
        if tx.send(StreamingSpeculativeBatchInput::Complete).is_err() {
            let result = scheduler
                .await
                .map_err(|e| Status::aborted(format!("speculative batch worker failed: {e}")))?;
            return match result {
                Ok(_) => Err(Status::aborted(
                    "speculative batch scheduler stopped before batch_end",
                )),
                Err(e) => Err(Status::aborted(format!("{:?}: {}", e.kind, e.detail))),
            };
        }
        drop(tx);

        debug!(
            "received speculative update client batch stream: client_id={} units={}",
            client_id, units
        );
        let result = scheduler
            .await
            .map_err(|e| Status::aborted(format!("speculative batch worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(encode_stitched_batch_result(res))),
            Err(e) => Err(Status::aborted(format!("{:?}: {}", e.kind, e.detail))),
        }
    }

    async fn aggregate_messages(
        &self,
        request: Request<MsgAggregateMessages>,
    ) -> Result<Response<MsgAggregateMessagesResponse>, Status> {
        let inner = request.into_inner();
        let app = self.app.clone();
        let result = tokio::task::spawn_blocking(move || {
            app.ecall_pool
                .run(move || app.enclave.proto_aggregate_messages(inner))
        })
        .await
        .map_err(|e| Status::aborted(format!("aggregate messages worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_membership(
        &self,
        request: Request<MsgVerifyMembership>,
    ) -> Result<Response<MsgVerifyMembershipResponse>, Status> {
        let inner = request.into_inner();
        let app = self.app.clone();
        let result = tokio::task::spawn_blocking(move || {
            app.ecall_pool
                .run(move || app.enclave.proto_verify_membership(inner))
        })
        .await
        .map_err(|e| Status::aborted(format!("verify membership worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_non_membership(
        &self,
        request: Request<MsgVerifyNonMembership>,
    ) -> Result<Response<MsgVerifyNonMembershipResponse>, Status> {
        let inner = request.into_inner();
        let app = self.app.clone();
        let result = tokio::task::spawn_blocking(move || {
            app.ecall_pool
                .run(move || app.enclave.proto_verify_non_membership(inner))
        })
        .await
        .map_err(|e| Status::aborted(format!("verify non-membership worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

#[tonic::async_trait]
impl<E, S> Query for AppService<E, S>
where
    S: CommitStore + TxAccessor + Send + 'static,
    E: EnclaveProtoAPI<S> + Send + Sync + 'static,
{
    async fn client(
        &self,
        request: Request<QueryClientRequest>,
    ) -> Result<Response<QueryClientResponse>, Status> {
        let inner = request.into_inner();
        let app = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            app.ecall_pool
                .run(move || app.enclave.proto_query_client(inner))
        })
        .await
        .map_err(|e| Status::aborted(format!("query client worker failed: {e}")))?;
        match result {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

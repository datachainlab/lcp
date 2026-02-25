use crate::service::AppService;
use core::fmt::Write;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::google::protobuf::Any;
use lcp_proto::lcp::service::elc::v1::msg_update_client_stream_chunk::Chunk;
use lcp_proto::lcp::service::elc::v1::{
    msg_server::Msg, query_server::Query, MsgAggregateMessages, MsgAggregateMessagesResponse,
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgUpdateClientStreamChunk, MsgVerifyMembership, MsgVerifyMembershipResponse,
    MsgVerifyNonMembership, MsgVerifyNonMembershipResponse, QueryClientRequest,
    QueryClientResponse,
};
use sha2::{Digest, Sha256};
use store::transaction::CommitStore;
use tonic::{Request, Response, Status, Streaming};

#[tonic::async_trait]
impl<E, S> Msg for AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    async fn create_client(
        &self,
        request: Request<MsgCreateClient>,
    ) -> Result<Response<MsgCreateClientResponse>, Status> {
        match self.enclave.proto_create_client(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn update_client(
        &self,
        request: Request<MsgUpdateClient>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        match self.enclave.proto_update_client(request.into_inner()) {
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
        let mut header_chunk_count = 0usize;

        while let Some(chunk_msg) = stream.message().await? {
            match chunk_msg.chunk {
                Some(Chunk::HeaderChunk(header_chunk)) => {
                    header_bytes.extend(header_chunk.data);
                    header_chunk_count += 1;
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

        let client_id = init.client_id.clone();
        let include_state = init.include_state;
        let type_url = init.type_url.clone();
        let header_len = header_bytes.len();
        let header_sha256 = sha256_hex(&header_bytes);
        log::debug!(
            "update_client_stream assembled: client_id={} include_state={} type_url={} chunk_count={} header_len={} header_sha256={}",
            client_id,
            include_state,
            type_url,
            header_chunk_count,
            header_len,
            header_sha256
        );

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

        match self.enclave.proto_update_client(msg) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => {
                log::error!(
                    "update_client_stream failed: client_id={} include_state={} type_url={} chunk_count={} header_len={} header_sha256={} err={}",
                    client_id,
                    include_state,
                    type_url,
                    header_chunk_count,
                    header_len,
                    header_sha256,
                    e
                );
                Err(Status::aborted(e.to_string()))
            }
        }
    }

    async fn aggregate_messages(
        &self,
        request: Request<MsgAggregateMessages>,
    ) -> Result<Response<MsgAggregateMessagesResponse>, Status> {
        match self.enclave.proto_aggregate_messages(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_membership(
        &self,
        request: Request<MsgVerifyMembership>,
    ) -> Result<Response<MsgVerifyMembershipResponse>, Status> {
        match self.enclave.proto_verify_membership(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_non_membership(
        &self,
        request: Request<MsgVerifyNonMembership>,
    ) -> Result<Response<MsgVerifyNonMembershipResponse>, Status> {
        match self
            .enclave
            .proto_verify_non_membership(request.into_inner())
        {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}

#[tonic::async_trait]
impl<E, S> Query for AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    async fn client(
        &self,
        request: Request<QueryClientRequest>,
    ) -> Result<Response<QueryClientResponse>, Status> {
        match self.enclave.proto_query_client(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

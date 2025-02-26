use crate::service::AppService;
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

        match self.enclave.proto_update_client(msg) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
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

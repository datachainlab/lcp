use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::elc::v1::{
    msg_server::Msg, query_server::Query, MsgCreateClient, MsgCreateClientResponse,
    MsgUpdateClient, MsgUpdateClientResponse, MsgVerifyChannel, MsgVerifyChannelResponse,
    MsgVerifyClient, MsgVerifyClientConsensus, MsgVerifyClientConsensusResponse,
    MsgVerifyClientResponse, MsgVerifyConnection, MsgVerifyConnectionResponse, QueryClientRequest,
    QueryClientResponse,
};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Msg for AppService {
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

    async fn verify_client(
        &self,
        request: Request<MsgVerifyClient>,
    ) -> Result<Response<MsgVerifyClientResponse>, Status> {
        match self.enclave.proto_verify_client(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_client_consensus(
        &self,
        request: Request<MsgVerifyClientConsensus>,
    ) -> Result<Response<MsgVerifyClientConsensusResponse>, Status> {
        match self
            .enclave
            .proto_verify_client_consensus(request.into_inner())
        {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_connection(
        &self,
        request: Request<MsgVerifyConnection>,
    ) -> Result<Response<MsgVerifyConnectionResponse>, Status> {
        match self.enclave.proto_verify_connection(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_channel(
        &self,
        request: Request<MsgVerifyChannel>,
    ) -> Result<Response<MsgVerifyChannelResponse>, Status> {
        match self.enclave.proto_verify_channel(request.into_inner()) {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

#[tonic::async_trait]
impl Query for AppService {
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

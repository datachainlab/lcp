use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::elc::v1::{
    msg_server::Msg, query_server::Query, MsgAggregateMessages, MsgAggregateMessagesResponse,
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyMembership, MsgVerifyMembershipResponse, MsgVerifyNonMembership,
    MsgVerifyNonMembershipResponse, QueryClientRequest, QueryClientResponse,
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

    async fn streaming_update_client(
        &self,
        request: Request<Streaming<MsgUpdateClient>>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        let mut complete = MsgUpdateClient {
            signer: vec![],
            client_id: "".to_string(),
            include_state: false,
            header: None,
        };

        let mut stream = request.into_inner();
        while let Some(chunk) = stream.message().await? {
            if let Some(header) = &mut complete.header {
                let any_header = chunk
                    .header
                    .ok_or(Status::invalid_argument("header value is required"))?;
                header.value.extend(any_header.value);
            } else {
                complete = chunk;
            }
        }

        match self.enclave.proto_update_client(complete) {
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

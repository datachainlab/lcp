use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use ibc_proto::ibc::core::client::v1::{
    msg_server::Msg, MsgCreateClient, MsgCreateClientResponse, MsgSubmitMisbehaviour,
    MsgSubmitMisbehaviourResponse, MsgUpdateClient, MsgUpdateClientResponse, MsgUpgradeClient,
    MsgUpgradeClientResponse,
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

    async fn upgrade_client(
        &self,
        _: Request<MsgUpgradeClient>,
    ) -> Result<Response<MsgUpgradeClientResponse>, Status> {
        todo!()
    }

    async fn submit_misbehaviour(
        &self,
        _: Request<MsgSubmitMisbehaviour>,
    ) -> Result<Response<MsgSubmitMisbehaviourResponse>, Status> {
        todo!()
    }
}

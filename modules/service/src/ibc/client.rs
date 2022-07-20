use crate::service::AppService;
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
        _: Request<MsgCreateClient>,
    ) -> Result<Response<MsgCreateClientResponse>, Status> {
        todo!()
    }

    async fn update_client(
        &self,
        _: Request<MsgUpdateClient>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        todo!()
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

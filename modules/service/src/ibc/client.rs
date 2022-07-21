use crate::service::AppService;
use enclave_api::EnclaveAPI;
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
        let req = request.into_inner();
        if req.client_state.is_none() || req.consensus_state.is_none() {
            return Err(Status::invalid_argument(
                "client_state and consensus_state must be non-nil",
            ));
        }
        let any_client_state = req.client_state.unwrap();
        let any_consensus_state = req.consensus_state.unwrap();
        match self.enclave.init_client(
            "unknown",
            any_client_state.into(),
            any_consensus_state.into(),
        ) {
            Ok(_) => Ok(Response::new(MsgCreateClientResponse {})),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
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

use crate::service::AppService;
use enclave_api::EnclaveAPI;
use ibc::core::ics24_host::identifier::ClientId;
use ibc_proto::ibc::core::client::v1::{
    msg_server::Msg, MsgCreateClient, MsgCreateClientResponse, MsgSubmitMisbehaviour,
    MsgSubmitMisbehaviourResponse, MsgUpdateClient, MsgUpdateClientResponse, MsgUpgradeClient,
    MsgUpgradeClientResponse,
};
use std::str::FromStr;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Msg for AppService {
    async fn create_client(
        &self,
        request: Request<MsgCreateClient>,
    ) -> Result<Response<MsgCreateClientResponse>, Status> {
        let req = request.into_inner();
        let any_client_state = req
            .client_state
            .ok_or_else(|| Status::invalid_argument("client_state must be non-nil"))?;
        let any_consensus_state = req
            .consensus_state
            .ok_or_else(|| Status::invalid_argument("consensus_state must be non-nil"))?;
        match self
            .enclave
            .init_client(any_client_state.into(), any_consensus_state.into())
        {
            Ok(_) => Ok(Response::new(MsgCreateClientResponse {})),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn update_client(
        &self,
        request: Request<MsgUpdateClient>,
    ) -> Result<Response<MsgUpdateClientResponse>, Status> {
        let req = request.into_inner();
        let header = req
            .header
            .ok_or_else(|| Status::invalid_argument("header must be non-nil"))?;
        let client_id = ClientId::from_str(&req.client_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        match self.enclave.update_client(client_id, header.into()) {
            Ok(_) => Ok(Response::new(MsgUpdateClientResponse {})),
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

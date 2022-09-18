use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::ibc::v1::{
    msg_server::Msg, MsgVerificationResponse, MsgVerifyChannel, MsgVerifyClient,
    MsgVerifyClientConsensus, MsgVerifyConnection, MsgVerifyNextSequenceRecv, MsgVerifyPacket,
    MsgVerifyPacketAcknowledgement, MsgVerifyPacketReceiptAbsense,
};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Msg for AppService {
    async fn verify_client(
        &self,
        request: Request<MsgVerifyClient>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_client_consensus(
        &self,
        request: Request<MsgVerifyClientConsensus>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_connection(
        &self,
        request: Request<MsgVerifyConnection>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_channel(
        &self,
        request: Request<MsgVerifyChannel>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_packet(
        &self,
        request: Request<MsgVerifyPacket>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_packet_acknowledgement(
        &self,
        request: Request<MsgVerifyPacketAcknowledgement>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_packet_receipt_absense(
        &self,
        request: Request<MsgVerifyPacketReceiptAbsense>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_non_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn verify_next_sequence_recv(
        &self,
        request: Request<MsgVerifyNextSequenceRecv>,
    ) -> Result<Response<MsgVerificationResponse>, Status> {
        match self
            .enclave
            .proto_verify_membership(request.into_inner().try_into()?)
        {
            Ok(res) => Ok(Response::new(res.into())),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

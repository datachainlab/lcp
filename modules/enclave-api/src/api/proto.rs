use crate::{Enclave, Result};
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyChannel, MsgVerifyChannelResponse, MsgVerifyClient, MsgVerifyClientConsensus,
    MsgVerifyClientConsensusResponse, MsgVerifyClientResponse, MsgVerifyConnection,
    MsgVerifyConnectionResponse, QueryClientRequest, QueryClientResponse,
};

use super::primitive::EnclavePrimitiveAPI;

pub trait EnclaveProtoAPI: EnclavePrimitiveAPI {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        Ok(self.init_client(msg.try_into()?)?.into())
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        Ok(self.update_client(msg.try_into()?)?.into())
    }

    fn proto_verify_client(&self, msg: MsgVerifyClient) -> Result<MsgVerifyClientResponse> {
        Ok(self.verify_client(msg.try_into()?)?.into())
    }

    fn proto_verify_client_consensus(
        &self,
        msg: MsgVerifyClientConsensus,
    ) -> Result<MsgVerifyClientConsensusResponse> {
        Ok(self.verify_client_consensus(msg.try_into()?)?.into())
    }

    fn proto_verify_connection(
        &self,
        msg: MsgVerifyConnection,
    ) -> Result<MsgVerifyConnectionResponse> {
        Ok(self.verify_connection(msg.try_into()?)?.into())
    }

    fn proto_verify_channel(&self, msg: MsgVerifyChannel) -> Result<MsgVerifyChannelResponse> {
        Ok(self.verify_channel(msg.try_into()?)?.into())
    }

    fn proto_query_client(&self, query: QueryClientRequest) -> Result<QueryClientResponse> {
        let res = self.query_client(query.try_into()?)?;
        Ok(QueryClientResponse {
            client_state: Some(res.any_client_state.into()),
            consensus_state: Some(res.any_consensus_state.into()),
        })
    }
}

impl EnclaveProtoAPI for Enclave {}

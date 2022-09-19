use crate::{Enclave, Result};
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyMembership, MsgVerifyMembershipResponse, MsgVerifyNonMembership,
    MsgVerifyNonMembershipResponse, QueryClientRequest, QueryClientResponse,
};

use super::primitive::EnclavePrimitiveAPI;

pub trait EnclaveProtoAPI: EnclavePrimitiveAPI {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        Ok(self.init_client(msg.try_into()?)?.into())
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        Ok(self.update_client(msg.try_into()?)?.into())
    }

    fn proto_verify_membership(
        &self,
        msg: MsgVerifyMembership,
    ) -> Result<MsgVerifyMembershipResponse> {
        Ok(self.verify_membership(msg.try_into()?)?.into())
    }

    fn proto_verify_non_membership(
        &self,
        msg: MsgVerifyNonMembership,
    ) -> Result<MsgVerifyNonMembershipResponse> {
        Ok(self.verify_non_membership(msg.try_into()?)?.into())
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

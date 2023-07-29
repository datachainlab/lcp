use super::command::EnclaveCommandAPI;
use crate::Result;
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyMembership, MsgVerifyMembershipResponse, MsgVerifyNonMembership,
    MsgVerifyNonMembershipResponse, QueryClientRequest, QueryClientResponse,
};
use log::*;
use store::transaction::CommitStore;

pub trait EnclaveProtoAPI<S: CommitStore>: EnclaveCommandAPI<S> {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        let res = self.init_client(msg.try_into()?)?;
        info!(
            "create_client: client_id={} commitment={{{}}}",
            res.client_id,
            res.proof.commitment()?
        );
        Ok(res.into())
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        let client_id = msg.client_id.clone();
        let res = self.update_client(msg.try_into()?)?;
        info!(
            "update_client: client_id={} commitment={{{}}}",
            client_id,
            res.0.commitment()?
        );
        Ok(res.into())
    }

    fn proto_verify_membership(
        &self,
        msg: MsgVerifyMembership,
    ) -> Result<MsgVerifyMembershipResponse> {
        let client_id = msg.client_id.clone();
        let res = self.verify_membership(msg.try_into()?)?;
        info!(
            "verify_membership: client_id={} commitment={{{}}}",
            client_id,
            res.0.commitment()?
        );
        Ok(res.into())
    }

    fn proto_verify_non_membership(
        &self,
        msg: MsgVerifyNonMembership,
    ) -> Result<MsgVerifyNonMembershipResponse> {
        let client_id = msg.client_id.clone();
        let res = self.verify_non_membership(msg.try_into()?)?;
        info!(
            "verify_non_membership: client_id={} commitment={{{}}}",
            client_id,
            res.0.commitment()?
        );
        Ok(res.into())
    }

    fn proto_query_client(&self, query: QueryClientRequest) -> Result<QueryClientResponse> {
        Ok(self.query_client(query.try_into()?)?.into())
    }
}

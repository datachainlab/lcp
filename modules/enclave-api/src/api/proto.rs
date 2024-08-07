use super::command::EnclaveCommandAPI;
use crate::Result;
use lcp_proto::lcp::service::elc::v1::{
    MsgAggregateMessages, MsgAggregateMessagesResponse, MsgCreateClient, MsgCreateClientResponse,
    MsgUpdateClient, MsgUpdateClientResponse, MsgVerifyMembership, MsgVerifyMembershipResponse,
    MsgVerifyNonMembership, MsgVerifyNonMembershipResponse, QueryClientRequest,
    QueryClientResponse,
};
use log::*;
use store::transaction::CommitStore;

pub trait EnclaveProtoAPI<S: CommitStore>: EnclaveCommandAPI<S> {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        let client_id = msg.client_id.clone();
        let res = self.init_client(msg.try_into()?)?;
        info!(
            "create_client: client_id={} message={{{}}}",
            client_id,
            res.proof.message()?
        );
        Ok(res.into())
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        let client_id = msg.client_id.clone();
        let res = self.update_client(msg.try_into()?)?;
        info!(
            "update_client: client_id={} message={{{}}}",
            client_id,
            res.0.message()?
        );
        Ok(res.into())
    }

    fn proto_aggregate_messages(
        &self,
        msg: MsgAggregateMessages,
    ) -> Result<MsgAggregateMessagesResponse> {
        let res = self.aggregate_messages(msg.try_into()?)?;
        info!("aggregate_commitments: message={{{}}}", res.0.message()?);
        Ok(res.into())
    }

    fn proto_verify_membership(
        &self,
        msg: MsgVerifyMembership,
    ) -> Result<MsgVerifyMembershipResponse> {
        let client_id = msg.client_id.clone();
        let res = self.verify_membership(msg.try_into()?)?;
        info!(
            "verify_membership: client_id={} message={{{}}}",
            client_id,
            res.0.message()?
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
            "verify_non_membership: client_id={} message={{{}}}",
            client_id,
            res.0.message()?
        );
        Ok(res.into())
    }

    fn proto_query_client(&self, query: QueryClientRequest) -> Result<QueryClientResponse> {
        Ok(self.query_client(query.try_into()?)?.into())
    }
}

use crate::{Enclave, Result};
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    QueryClientRequest, QueryClientResponse,
};

use super::primitive::EnclavePrimitiveAPI;

pub trait EnclaveProtoAPI: EnclavePrimitiveAPI {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        let res = self.init_client(msg.try_into()?)?;
        let proof = res.proof;
        Ok(MsgCreateClientResponse {
            client_id: res.client_id.to_string(),
            commitment: proof.commitment().to_vec(),
            signer: proof.signer,
            signature: proof.signature,
        })
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        let proof = self.update_client(msg.try_into()?)?.0;
        Ok(MsgUpdateClientResponse {
            commitment: proof.commitment().to_vec(),
            signer: proof.signer,
            signature: proof.signature,
        })
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

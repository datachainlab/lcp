use crate::{Enclave, EnclaveAPIError, Result};
use ibc::core::ics24_host::identifier::ClientId;
use lcp_proto::lcp::service::{
    elc::v1::{MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse},
    enclave::v1::{QueryAttestedEnclaveKeyRequest, QueryAttestedEnclaveKeyResponse},
};
use std::str::FromStr;

use super::primitive::EnclavePrimitiveAPI;

pub trait EnclaveProtoAPI: EnclavePrimitiveAPI {
    fn proto_create_client(&self, msg: MsgCreateClient) -> Result<MsgCreateClientResponse> {
        let any_client_state = msg.client_state.ok_or_else(|| {
            EnclaveAPIError::InvalidArgumentError("client_state must be non-nil".into())
        })?;
        let any_consensus_state = msg.consensus_state.ok_or_else(|| {
            EnclaveAPIError::InvalidArgumentError("consensus_state must be non-nil".into())
        })?;

        let res = self.init_client(any_client_state.into(), any_consensus_state.into())?;
        let proof = res.proof;
        Ok(MsgCreateClientResponse {
            client_id: res.client_id.to_string(),
            commitment: proof.commitment().to_vec(),
            signer: proof.signer,
            signature: proof.signature,
        })
    }

    fn proto_update_client(&self, msg: MsgUpdateClient) -> Result<MsgUpdateClientResponse> {
        let header = msg.header.ok_or_else(|| {
            EnclaveAPIError::InvalidArgumentError("header must be non-nil".into())
        })?;
        let client_id = ClientId::from_str(&msg.client_id)
            .map_err(|e| EnclaveAPIError::InvalidArgumentError(e.to_string()))?;

        let proof = self.update_client(client_id, header.into())?.0;
        Ok(MsgUpdateClientResponse {
            commitment: proof.commitment().to_vec(),
            signer: proof.signer,
            signature: proof.signature,
        })
    }

    fn proto_query_attested_enclave_key(
        &self,
        request: QueryAttestedEnclaveKeyRequest,
    ) -> Result<QueryAttestedEnclaveKeyResponse> {
        todo!()
    }
}

impl EnclaveProtoAPI for Enclave {}

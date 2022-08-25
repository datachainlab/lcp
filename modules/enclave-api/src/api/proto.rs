use crate::{Enclave, EnclaveAPIError, Result};
use enclave_commands::{CommandResult, LightClientResult};
use ibc::core::ics24_host::identifier::ClientId;
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
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

        let proof = if let CommandResult::LightClient(LightClientResult::InitClient(res)) =
            self.init_client(any_client_state.into(), any_consensus_state.into())?
        {
            res.0
        } else {
            unreachable!()
        };
        let commitment = proof.commitment();
        Ok(MsgCreateClientResponse {
            client_id: commitment.client_id.to_string(),
            commitment: commitment.to_vec(),
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

        let proof = if let CommandResult::LightClient(LightClientResult::UpdateClient(res)) =
            self.update_client(client_id, header.into())?
        {
            res.0
        } else {
            unreachable!()
        };
        Ok(MsgUpdateClientResponse {
            commitment: proof.commitment().to_vec(),
            signer: proof.signer,
            signature: proof.signature,
        })
    }
}

impl EnclaveProtoAPI for Enclave {}

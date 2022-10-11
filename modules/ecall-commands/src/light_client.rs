use crate::errors::ECallCommandError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::{StateCommitmentProof, UpdateClientCommitmentProof};
use core::convert::TryFrom;
use core::str::FromStr;
use ibc::core::ics24_host::identifier::ClientId;
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyMembership, MsgVerifyMembershipResponse, MsgVerifyNonMembership,
    MsgVerifyNonMembershipResponse, QueryClientRequest, QueryClientResponse,
};
use lcp_types::{Any, Height, Time};
use serde::{Deserialize, Serialize};
use std::string::{String, ToString};
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientCommand {
    InitClient(InitClientInput),
    UpdateClient(UpdateClientInput),

    VerifyMembership(VerifyMembershipInput),
    VerifyNonMembership(VerifyNonMembershipInput),

    QueryClient(QueryClientInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientInput {
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub current_timestamp: Time,
}

impl TryFrom<MsgCreateClient> for InitClientInput {
    type Error = Error;
    fn try_from(msg: MsgCreateClient) -> Result<Self, Error> {
        let any_client_state = msg
            .client_state
            .ok_or_else(|| Error::InvalidArgumentError("client_state must be non-nil".into()))?
            .into();
        let any_consensus_state = msg
            .consensus_state
            .ok_or_else(|| Error::InvalidArgumentError("consensus_state must be non-nil".into()))?
            .into();
        Ok(Self {
            any_client_state,
            any_consensus_state,
            current_timestamp: Time::now(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientInput {
    pub client_id: ClientId,
    pub any_header: Any,
    pub include_state: bool,
    pub current_timestamp: Time,
}

impl TryFrom<MsgUpdateClient> for UpdateClientInput {
    type Error = Error;
    fn try_from(msg: MsgUpdateClient) -> Result<Self, Error> {
        let any_header = msg
            .header
            .ok_or_else(|| Error::InvalidArgumentError("header must be non-nil".into()))?
            .into();
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        Ok(Self {
            client_id,
            any_header,
            include_state: msg.include_state,
            current_timestamp: Time::now(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyMembershipInput {
    pub client_id: ClientId,
    pub prefix: Vec<u8>,
    pub path: String,
    pub value: Vec<u8>,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyMembership> for VerifyMembershipInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyMembership) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            prefix: msg.prefix,
            proof,
            path: msg.path,
            value: msg.value,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyNonMembershipInput {
    pub client_id: ClientId,
    pub prefix: Vec<u8>,
    pub path: String,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyNonMembership> for VerifyNonMembershipInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyNonMembership) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            prefix: msg.prefix,
            proof,
            path: msg.path,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitmentProofPair(pub Height, pub Vec<u8>);

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientInput {
    pub client_id: ClientId,
}

impl TryFrom<QueryClientRequest> for QueryClientInput {
    type Error = Error;
    fn try_from(query: QueryClientRequest) -> Result<Self, Error> {
        let client_id =
            ClientId::from_str(&query.client_id).map_err(Error::ICS24ValidationError)?;
        Ok(Self { client_id })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientResult {
    InitClient(InitClientResult),
    UpdateClient(UpdateClientResult),

    VerifyMembership(VerifyMembershipResult),
    VerifyNonMembership(VerifyNonMembershipResult),

    QueryClient(QueryClientResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientResult {
    pub client_id: ClientId,
    pub proof: UpdateClientCommitmentProof,
}

impl From<InitClientResult> for MsgCreateClientResponse {
    fn from(res: InitClientResult) -> Self {
        Self {
            client_id: res.client_id.to_string(),
            commitment: res.proof.commitment_bytes,
            signer: res.proof.signer,
            signature: res.proof.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UpdateClientResult(pub UpdateClientCommitmentProof);

impl From<UpdateClientResult> for MsgUpdateClientResponse {
    fn from(res: UpdateClientResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyMembershipResult(pub StateCommitmentProof);

impl From<VerifyMembershipResult> for MsgVerifyMembershipResponse {
    fn from(res: VerifyMembershipResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyNonMembershipResult(pub StateCommitmentProof);

impl From<VerifyNonMembershipResult> for MsgVerifyNonMembershipResponse {
    fn from(res: VerifyNonMembershipResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientResult {
    pub any_client_state: Any,
    pub any_consensus_state: Any,
}

impl From<QueryClientResult> for QueryClientResponse {
    fn from(res: QueryClientResult) -> Self {
        Self {
            client_state: Some(res.any_client_state.into()),
            consensus_state: Some(res.any_consensus_state.into()),
        }
    }
}

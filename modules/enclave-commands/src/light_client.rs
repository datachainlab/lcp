use crate::errors::EnclaveCommandError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::{StateCommitmentProof, UpdateClientCommitmentProof};
use core::convert::{TryFrom, TryInto};
use core::str::FromStr;
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use lcp_proto::lcp::service::elc::v1::{
    MsgCreateClient, MsgCreateClientResponse, MsgUpdateClient, MsgUpdateClientResponse,
    MsgVerifyChannel, MsgVerifyChannelResponse, MsgVerifyClient, MsgVerifyClientConsensus,
    MsgVerifyClientConsensusResponse, MsgVerifyClientResponse, MsgVerifyConnection,
    MsgVerifyConnectionResponse, QueryClientRequest, QueryClientResponse,
};
use lcp_types::{Any, Height, Time};
use serde::{Deserialize, Serialize};
use std::string::ToString;
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientCommand {
    InitClient(InitClientInput),
    UpdateClient(UpdateClientInput),
    VerifyClient(VerifyClientInput),
    VerifyClientConsensus(VerifyClientConsensusInput),
    VerifyConnection(VerifyConnectionInput),
    VerifyChannel(VerifyChannelInput),

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
            current_timestamp: Time::now(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientInput {
    pub client_id: ClientId,
    pub target_any_client_state: Any,
    pub prefix: Vec<u8>,
    pub counterparty_client_id: ClientId,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyClient> for VerifyClientInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyClient) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let target_any_client_state = msg
            .target_any_client_state
            .ok_or_else(|| {
                Error::InvalidArgumentError("target_any_client_state must be non-nil".into())
            })?
            .into();
        let counterparty_client_id =
            ClientId::from_str(&msg.counterparty_client_id).map_err(Error::ICS24ValidationError)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            target_any_client_state,
            prefix: msg.prefix,
            counterparty_client_id,
            proof,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientConsensusInput {
    pub client_id: ClientId,
    pub target_any_client_consensus_state: Any,
    pub prefix: Vec<u8>,
    pub counterparty_client_id: ClientId,
    pub counterparty_consensus_height: Height,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyClientConsensus> for VerifyClientConsensusInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyClientConsensus) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let target_any_client_consensus_state = msg
            .target_any_client_consensus_state
            .ok_or_else(|| {
                Error::InvalidArgumentError(
                    "target_any_client_consensus_state must be non-nil".into(),
                )
            })?
            .into();
        let counterparty_client_id =
            ClientId::from_str(&msg.counterparty_client_id).map_err(Error::ICS24ValidationError)?;
        let counterparty_consensus_height = msg
            .counterparty_consensus_height
            .ok_or_else(|| {
                Error::InvalidArgumentError("counterparty_consensus_height must be non-nil".into())
            })?
            .into();
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            target_any_client_consensus_state,
            prefix: msg.prefix,
            counterparty_client_id,
            counterparty_consensus_height,
            proof,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConnectionInput {
    pub client_id: ClientId,
    pub expected_connection: ConnectionEnd,
    pub prefix: Vec<u8>,
    pub counterparty_connection_id: ConnectionId,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyConnection> for VerifyConnectionInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyConnection) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        let counterparty_connection_id = ConnectionId::from_str(&msg.counterparty_connection_id)
            .map_err(Error::ICS24ValidationError)?;
        let expected_connection: ConnectionEnd = msg
            .expected_connection
            .ok_or_else(|| {
                Error::InvalidArgumentError("expected_connection must be non-nil".into())
            })?
            .try_into()
            .map_err(Error::ICS03Error)?;
        Ok(Self {
            client_id,
            expected_connection,
            prefix: msg.prefix,
            counterparty_connection_id,
            proof,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyChannelInput {
    pub client_id: ClientId,
    pub expected_channel: ChannelEnd,
    pub prefix: Vec<u8>,
    pub counterparty_port_id: PortId,
    pub counterparty_channel_id: ChannelId,
    pub proof: CommitmentProofPair,
}

impl TryFrom<MsgVerifyChannel> for VerifyChannelInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyChannel) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id).map_err(Error::ICS24ValidationError)?;
        let counterparty_port_id =
            PortId::from_str(&msg.counterparty_port_id).map_err(Error::ICS24ValidationError)?;
        let counterparty_channel_id = ChannelId::from_str(&msg.counterparty_channel_id)
            .map_err(Error::ICS24ValidationError)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::InvalidArgumentError("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        let expected_channel: ChannelEnd = msg
            .expected_channel
            .ok_or_else(|| Error::InvalidArgumentError("expected_channel must be non-nil".into()))?
            .try_into()
            .map_err(Error::ICS04Error)?;
        Ok(Self {
            client_id,
            expected_channel,
            prefix: msg.prefix,
            counterparty_port_id,
            counterparty_channel_id,
            proof,
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
    VerifyClient(VerifyClientResult),
    VerifyClientConsensus(VerifyClientConsensusResult),
    VerifyConnection(VerifyConnectionResult),
    VerifyChannel(VerifyChannelResult),

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
pub struct VerifyClientResult(pub StateCommitmentProof);

impl From<VerifyClientResult> for MsgVerifyClientResponse {
    fn from(res: VerifyClientResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientConsensusResult(pub StateCommitmentProof);

impl From<VerifyClientConsensusResult> for MsgVerifyClientConsensusResponse {
    fn from(res: VerifyClientConsensusResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConnectionResult(pub StateCommitmentProof);

impl From<VerifyConnectionResult> for MsgVerifyConnectionResponse {
    fn from(res: VerifyConnectionResult) -> Self {
        Self {
            commitment: res.0.commitment_bytes,
            signer: res.0.signer,
            signature: res.0.signature,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyChannelResult(pub StateCommitmentProof);

impl From<VerifyChannelResult> for MsgVerifyChannelResponse {
    fn from(res: VerifyChannelResult) -> Self {
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

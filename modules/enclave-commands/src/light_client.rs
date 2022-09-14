use crate::errors::EnclaveCommandError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::{StateCommitmentProof, UpdateClientCommitmentProof};
use core::str::FromStr;
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use lcp_proto::lcp::service::elc::v1::{MsgCreateClient, MsgUpdateClient, QueryClientRequest};
use lcp_types::{Any, Height, Time};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientConsensusInput {
    pub client_id: ClientId,
    pub target_any_client_consensus_state: Any,
    pub prefix: Vec<u8>,
    pub counterparty_client_id: ClientId,
    pub counterparty_consensus_height: Height,
    pub proof: CommitmentProofPair,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConnectionInput {
    pub client_id: ClientId,
    pub expected_connection: ConnectionEnd,
    pub prefix: Vec<u8>,
    pub counterparty_connection_id: ConnectionId,
    pub proof: CommitmentProofPair,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UpdateClientResult(pub UpdateClientCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientResult(pub StateCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClientConsensusResult(pub StateCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConnectionResult(pub StateCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyChannelResult(pub StateCommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientResult {
    pub any_client_state: Any,
    pub any_consensus_state: Any,
}

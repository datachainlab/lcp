use crate::{prelude::*, EnclaveKeySelector};
use commitments::CommitmentProof;
use crypto::Address;
use lcp_types::{Any, ClientId, Height};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientCommand {
    Execute(LightClientExecuteCommand),
    Query(LightClientQueryCommand),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientExecuteCommand {
    InitClient(InitClientInput),
    UpdateClient(UpdateClientInput),
    AggregateMessages(AggregateMessagesInput),
    VerifyMembership(VerifyMembershipInput),
    VerifyNonMembership(VerifyNonMembershipInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientQueryCommand {
    QueryClient(QueryClientInput),
}

impl EnclaveKeySelector for LightClientCommand {
    fn get_enclave_key(&self) -> Option<Address> {
        match self {
            Self::Execute(cmd) => match cmd {
                LightClientExecuteCommand::InitClient(input) => Some(input.signer),
                LightClientExecuteCommand::UpdateClient(input) => Some(input.signer),
                LightClientExecuteCommand::AggregateMessages(input) => Some(input.signer),
                LightClientExecuteCommand::VerifyMembership(input) => Some(input.signer),
                LightClientExecuteCommand::VerifyNonMembership(input) => Some(input.signer),
            },
            Self::Query(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientInput {
    pub client_id: String,
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientInput {
    pub client_id: ClientId,
    pub any_header: Any,
    pub include_state: bool,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateMessagesInput {
    pub signer: Address,
    pub messages: Vec<Vec<u8>>,
    pub signatures: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyMembershipInput {
    pub client_id: ClientId,
    pub prefix: Vec<u8>,
    pub path: String,
    pub value: Vec<u8>,
    pub proof: CommitmentProofPair,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyNonMembershipInput {
    pub client_id: ClientId,
    pub prefix: Vec<u8>,
    pub path: String,
    pub proof: CommitmentProofPair,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitmentProofPair(pub Height, pub Vec<u8>);

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientInput {
    pub client_id: ClientId,
    /// Optional height for per-height canonical lookup (per-height client_state design).
    /// When `None`, the response carries the latest singleton entry — exactly
    /// matching pre-D behaviour for every existing caller.
    pub height: Option<Height>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientResponse {
    InitClient(InitClientResponse),
    UpdateClient(UpdateClientResponse),
    AggregateMessages(AggregateMessagesResponse),

    VerifyMembership(VerifyMembershipResponse),
    VerifyNonMembership(VerifyNonMembershipResponse),

    QueryClient(QueryClientResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientResponse {
    pub proof: CommitmentProof,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientResponse(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateMessagesResponse(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyMembershipResponse(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyNonMembershipResponse(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientResponse {
    pub found: bool,
    pub any_client_state: Option<Any>,
    pub any_consensus_state: Option<Any>,
    /// 32-byte canonical state_id at the resolved height (the explicit-state drift design
    ///). Empty for legacy entries that predate state_id tracking.
    pub state_id: Option<Vec<u8>>,
    /// Resolved height. Echoes the request `height` when supplied; otherwise
    /// the latest tip inferred from the returned `any_client_state`.
    /// `None` when `found = false`.
    pub height: Option<Height>,
}

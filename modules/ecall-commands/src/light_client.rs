use crate::{prelude::*, EnclaveKeySelector};
use commitments::CommitmentProof;
use crypto::Address;
use lcp_types::{Any, ClientId, Height, Time};
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
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub current_timestamp: Time,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientInput {
    pub client_id: ClientId,
    pub any_header: Any,
    pub include_state: bool,
    pub current_timestamp: Time,
    pub signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateMessagesInput {
    pub signer: Address,
    pub messages: Vec<Vec<u8>>,
    pub signatures: Vec<Vec<u8>>,
    pub current_timestamp: Time,
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
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LightClientResult {
    InitClient(InitClientResult),
    UpdateClient(UpdateClientResult),
    AggregateMessages(AggregateMessagesResult),

    VerifyMembership(VerifyMembershipResult),
    VerifyNonMembership(VerifyNonMembershipResult),

    QueryClient(QueryClientResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientResult {
    pub client_id: ClientId,
    pub proof: CommitmentProof,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClientResult(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateMessagesResult(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyMembershipResult(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyNonMembershipResult(pub CommitmentProof);

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryClientResult {
    pub any_client_state: Any,
    pub any_consensus_state: Any,
}

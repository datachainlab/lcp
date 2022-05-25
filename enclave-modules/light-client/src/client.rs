use crate::errors::Result;
use commitments::{StateCommitment, UpdateClientCommitment};
use ibc::{
    core::{
        ics02_client::{context::ClientReader, height::Height},
        ics03_connection::{connection::ConnectionEnd, context::ConnectionReader},
        ics04_channel::channel::ChannelEnd,
        ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId},
    },
    timestamp::Timestamp,
};
use prost_types::Any;
use std::string::String;
use std::vec::Vec;

pub trait LightClient {
    fn create_client(
        &self,
        ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult>;
    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult>;
    fn verify_client(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_client_state: Any,
        counterparty_prefix: Vec<u8>,
        counterparty_client_id: ClientId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult>;
    fn verify_client_consensus(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_client_consensus_state: Any,
        counterparty_prefix: Vec<u8>,
        counterparty_client_id: ClientId,
        counterparty_consensus_height: Height,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult>;
    fn verify_connection(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_connection_state: ConnectionEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_connection_id: ConnectionId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult>;
    fn verify_channel(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_channel_state: ChannelEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_port_id: PortId,
        counterparty_channel_id: ChannelId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateClientResult {
    pub client_id: ClientId,
    pub client_type: String,
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub height: Height,
    pub timestamp: Timestamp,
    pub commitment: UpdateClientCommitment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateClientResult {
    pub client_id: ClientId,
    pub new_any_client_state: Any,
    pub new_any_consensus_state: Any,
    pub height: Height,
    pub timestamp: Timestamp,
    pub commitment: UpdateClientCommitment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateVerificationResult {
    pub state_commitment: StateCommitment,
}

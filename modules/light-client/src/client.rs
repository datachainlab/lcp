#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::LightClientError;
use commitments::{StateCommitment, UpdateClientCommitment};
use ibc::{
    core::{
        ics02_client::{context::ClientReader, error::Error as ICS02Error, height::Height},
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
        ctx: &dyn LightClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError>;
    fn update_client(
        &self,
        ctx: &dyn LightClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError>;
    fn verify_client(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_client_state: Any,
        counterparty_prefix: Vec<u8>,
        counterparty_client_id: ClientId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError>;
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
    ) -> Result<StateVerificationResult, LightClientError>;
    fn verify_connection(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_connection_state: ConnectionEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_connection_id: ConnectionId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError>;
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
    ) -> Result<StateVerificationResult, LightClientError>;
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

pub trait LightClientReader {
    fn client_type(&self, client_id: &ClientId) -> Result<String, ICS02Error>;
    fn client_state(&self, client_id: &ClientId) -> Result<Any, ICS02Error>;
    fn consensus_state(&self, client_id: &ClientId, height: Height) -> Result<Any, ICS02Error>;
    fn host_height(&self) -> Height;
    fn host_timestamp(&self) -> Timestamp;
    fn as_client_reader(&self) -> &dyn ClientReader;
}

pub trait LightClientKeeper {
    /// Called upon successful client creation
    fn store_client_type(
        &mut self,
        client_id: ClientId,
        client_type: String,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client creation and update
    fn store_any_client_state(
        &mut self,
        client_id: ClientId,
        client_state: Any,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client creation and update
    fn store_any_consensus_state(
        &mut self,
        client_id: ClientId,
        height: Height,
        consensus_state: Any,
    ) -> Result<(), ICS02Error>;

    /// Called upon client creation.
    /// Increases the counter which keeps track of how many clients have been created.
    /// Should never fail.
    fn increase_client_counter(&mut self);

    /// Called upon successful client update.
    /// Implementations are expected to use this to record the specified time as the time at which
    /// this update (or header) was processed.
    fn store_update_time(
        &mut self,
        client_id: ClientId,
        height: Height,
        timestamp: Timestamp,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client update.
    /// Implementations are expected to use this to record the specified height as the height at
    /// at which this update (or header) was processed.
    fn store_update_height(
        &mut self,
        client_id: ClientId,
        height: Height,
        host_height: Height,
    ) -> Result<(), ICS02Error>;
}

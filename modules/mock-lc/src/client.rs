use crate::errors::MockLCError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id, gen_state_id_from_any, StateCommitment, UpdateClientCommitment};
use ibc::core::ics02_client::client_consensus::{AnyConsensusState, ConsensusState};
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{AnyClientState, ClientState};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::context::ClientReader;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics03_connection::context::ConnectionReader;
use ibc::core::ics03_connection::error::Error as ICS03Error;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics04_channel::error::Error as ICS04Error;
use ibc::core::ics23_commitment::commitment::{CommitmentPrefix, CommitmentProofBytes};
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::core::ics24_host::path::{
    ChannelEndsPath, ClientConsensusStatePath, ClientStatePath, ConnectionsPath,
};
use ibc::core::ics24_host::Path;
use ibc::Height;
use light_client::LightClientError;
use light_client::{CreateClientResult, StateVerificationResult, UpdateClientResult};
use light_client::{LightClient, LightClientRegistry};
use log::*;
use prost_types::Any;
use serde_json::Value;
use std::boxed::Box;
use std::string::{String, ToString};
use std::vec::Vec;

#[derive(Default)]
pub struct MockLightClient;

impl LightClient for MockLightClient {
    fn create_client(
        &self,
        ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        todo!()
    }

    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        todo!()
    }

    fn verify_client(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_client_state: Any,
        counterparty_prefix: Vec<u8>,
        counterparty_client_id: ClientId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

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
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

    fn verify_connection(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_connection_state: ConnectionEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_connection_id: ConnectionId,
        proof_height: ibc::core::ics02_client::height::Height,
        proof: Vec<u8>,
    ) -> light_client::Result<StateVerificationResult> {
        todo!()
    }

    fn verify_channel(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_channel_state: ChannelEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_port_id: PortId,
        counterparty_channel_id: ChannelId,
        proof_height: ibc::core::ics02_client::height::Height,
        proof: Vec<u8>,
    ) -> light_client::Result<StateVerificationResult> {
        todo!()
    }
}

pub fn register_implementations(registry: &mut LightClientRegistry) {
    registry
        .put(
            ClientType::Mock.as_str().to_string(),
            Box::new(MockLightClient),
        )
        .unwrap()
}

use crate::errors::MockLCError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id, gen_state_id_from_any, UpdateClientCommitment};
use ibc::core::ics02_client::client_consensus::AnyConsensusState;
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{
    AnyClientState, ClientState, MOCK_CLIENT_STATE_TYPE_URL,
};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use lcp_types::{Any, Height, Time};
use light_client::{
    ClientReader, CreateClientResult, LightClient, LightClientError, LightClientRegistry,
    StateVerificationResult, UpdateClientResult,
};
use std::boxed::Box;
use std::string::ToString;
use std::vec::Vec;
use validation_context::ValidationParams;

#[derive(Default)]
pub struct MockLightClient;

impl LightClient for MockLightClient {
    fn create_client(
        &self,
        _: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let state_id = gen_state_id_from_any(&any_client_state, &any_consensus_state)
            .map_err(Error::OtherError)?;
        let client_state = match AnyClientState::try_from(any_client_state.clone()) {
            Ok(AnyClientState::Mock(client_state)) => AnyClientState::Mock(client_state),
            #[allow(unreachable_patterns)]
            Ok(s) => {
                return Err(Error::UnexpectedClientTypeError(s.client_type().to_string()).into())
            }
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };
        let consensus_state = match AnyConsensusState::try_from(any_consensus_state.clone()) {
            Ok(AnyConsensusState::Mock(consensus_state)) => {
                AnyConsensusState::Mock(consensus_state)
            }
            #[allow(unreachable_patterns)]
            Ok(s) => {
                return Err(Error::UnexpectedClientTypeError(s.client_type().to_string()).into())
            }
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };

        let height = client_state.latest_height().into();
        let timestamp: Time = consensus_state.timestamp().into();
        Ok(CreateClientResult {
            any_client_state: any_client_state.clone(),
            any_consensus_state,
            height,
            timestamp,
            commitment: UpdateClientCommitment {
                prev_state_id: None,
                new_state_id: state_id,
                new_state: Some(any_client_state.into()),
                prev_height: None,
                new_height: height,
                timestamp,
                validation_params: ValidationParams::Empty,
            },
        })
    }

    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let ctx = ctx.as_ibc_client_reader();
        let header = match AnyHeader::try_from(any_header) {
            Ok(AnyHeader::Mock(header)) => AnyHeader::Mock(header),
            #[allow(unreachable_patterns)]
            Ok(h) => {
                return Err(Error::UnexpectedClientTypeError(h.client_type().to_string()).into())
            }
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };

        // Read client type from the host chain store. The client should already exist.
        let client_type = ctx.client_type(&client_id).map_err(Error::ICS02Error)?;

        let client_def = AnyClient::from_client_type(client_type);

        // Read client state from the host chain store.
        let client_state = ctx.client_state(&client_id).map_err(Error::ICS02Error)?;

        if client_state.is_frozen() {
            return Err(Error::ICS02Error(ICS02Error::client_frozen(client_id)).into());
        }

        let height = header.height().into();
        let header_timestamp = header.timestamp().into();

        let latest_height = client_state.latest_height();

        // Read consensus state from the host chain store.
        let latest_consensus_state =
            ctx.consensus_state(&client_id, latest_height)
                .map_err(|_| {
                    Error::ICS02Error(ICS02Error::consensus_state_not_found(
                        client_id.clone(),
                        latest_height,
                    ))
                })?;

        // Use client_state to validate the new header against the latest consensus_state.
        // This function will return the new client_state (its latest_height changed) and a
        // consensus_state obtained from header. These will be later persisted by the keeper.
        let (new_client_state, new_consensus_state) = client_def
            .check_header_and_update_state(ctx, client_id.clone(), client_state.clone(), header)
            .map_err(|e| {
                Error::ICS02Error(ICS02Error::header_verification_failure(e.to_string()))
            })?;

        let prev_state_id =
            gen_state_id(client_state, latest_consensus_state).map_err(Error::OtherError)?;
        let new_state_id = gen_state_id(new_client_state.clone(), new_consensus_state.clone())
            .map_err(Error::OtherError)?;
        let new_any_client_state = Any::try_from(new_client_state).unwrap();

        Ok(UpdateClientResult {
            client_id,
            new_any_client_state: new_any_client_state.clone(),
            new_any_consensus_state: Any::try_from(new_consensus_state).unwrap(),
            height,
            timestamp: header_timestamp,
            commitment: UpdateClientCommitment {
                prev_state_id: Some(prev_state_id),
                new_state_id,
                new_state: new_any_client_state.into(),
                prev_height: Some(latest_height.into()),
                new_height: height,
                timestamp: header_timestamp,
                validation_params: ValidationParams::Empty,
            },
        })
    }

    fn verify_client(
        &self,
        ctx: &dyn ClientReader,
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
        ctx: &dyn ClientReader,
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
        ctx: &dyn ClientReader,
        client_id: ClientId,
        expected_connection_state: ConnectionEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_connection_id: ConnectionId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> light_client::Result<StateVerificationResult> {
        todo!()
    }

    fn verify_channel(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        expected_channel_state: ChannelEnd,
        counterparty_prefix: Vec<u8>,
        counterparty_port_id: PortId,
        counterparty_channel_id: ChannelId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> light_client::Result<StateVerificationResult> {
        todo!()
    }

    fn client_type(&self) -> String {
        ClientType::Mock.as_str().to_owned()
    }
}

pub fn register_implementations(registry: &mut LightClientRegistry) {
    registry
        .put(
            MOCK_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(MockLightClient),
        )
        .unwrap()
}

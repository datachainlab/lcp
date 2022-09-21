use super::errors::LCPLCError as Error;
use crate::client_def::LCPClient;
use crate::client_state::{ClientState, LCP_CLIENT_STATE_TYPE_URL};
use crate::consensus_state::ConsensusState;
use crate::header::Header;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id_from_any, UpdateClientCommitment};
use ibc::core::ics02_client::client_state::ClientState as ICS02ClientState;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics24_host::identifier::ClientId;
use lcp_types::{Any, Height};
use light_client::{ClientReader, LightClientError};
use light_client::{CreateClientResult, StateVerificationResult, UpdateClientResult};
use light_client::{LightClient, LightClientRegistry};
use std::boxed::Box;
use std::string::ToString;
use std::vec::Vec;
use validation_context::ValidationParams;

pub const LCP_CLIENT_TYPE: &str = "0000-lcp";

#[derive(Default)]
pub struct LCPLightClient;

// WARNING: This implementation is intended for testing purpose only
// each function always returns the default value as a commitment
impl LightClient for LCPLightClient {
    fn create_client(
        &self,
        _ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let state_id = gen_state_id_from_any(&any_client_state, &any_consensus_state)
            .map_err(LightClientError::OtherError)?;
        let client_state =
            ClientState::try_from(any_client_state).map_err(LightClientError::ICS02Error)?;
        let consensus_state =
            ConsensusState::try_from(any_consensus_state).map_err(LightClientError::ICS02Error)?;
        let height = client_state.latest_height().into();
        let timestamp = consensus_state.timestamp;

        LCPClient {}
            .initialise(&client_state, &consensus_state)
            .map_err(LightClientError::ICS02Error)?;

        Ok(CreateClientResult {
            any_client_state: client_state.clone().into(),
            any_consensus_state: consensus_state.into(),
            height,
            timestamp,
            commitment: UpdateClientCommitment {
                prev_state_id: None,
                new_state_id: state_id,
                new_state: Some(client_state.into()),
                prev_height: None,
                new_height: height,
                timestamp,
                validation_params: ValidationParams::Empty,
            },
            prove: false,
        })
    }

    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let header = Header::try_from(any_header).map_err(LightClientError::ICS02Error)?;

        // Read client type from the host chain store. The client should already exist.
        let client_type = ctx
            .client_type(&client_id)
            .map_err(LightClientError::ICS02Error)?;

        assert!(client_type.eq(LCP_CLIENT_TYPE));

        // Read client state from the host chain store.
        let client_state = ctx
            .client_state(&client_id)
            .map_err(LightClientError::ICS02Error)?
            .try_into()
            .map_err(LightClientError::ICS02Error)?;

        // if client_state.is_frozen() {
        //     return Err(Error::ICS02Error(ICS02Error::client_frozen(client_id)).into());
        // }

        let height = header.get_height().unwrap_or_default();
        let header_timestamp = header.get_timestamp().unwrap();

        // Use client_state to validate the new header against the latest consensus_state.
        // This function will return the new client_state (its latest_height changed) and a
        // consensus_state obtained from header. These will be later persisted by the keeper.
        let (new_client_state, new_consensus_state) = LCPClient {}
            .check_header_and_update_state(ctx, client_id, client_state, header)
            .map_err(|e| {
                Error::ICS02Error(ICS02Error::header_verification_failure(e.to_string()))
            })?;

        Ok(UpdateClientResult {
            new_any_client_state: Any::new(new_client_state),
            new_any_consensus_state: Any::new(new_consensus_state),
            height,
            timestamp: header_timestamp,
            commitment: UpdateClientCommitment::default(),
            prove: true,
        })
    }

    fn client_type(&self) -> String {
        LCP_CLIENT_TYPE.to_owned()
    }

    fn latest_height(
        &self,
        ctx: &dyn ClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state: ClientState = ctx
            .client_state(&client_id)
            .map_err(LightClientError::ICS02Error)?
            .try_into()
            .map_err(LightClientError::ICS02Error)?;
        Ok(client_state.latest_height)
    }

    fn verify_membership(
        &self,
        _ctx: &dyn ClientReader,
        _client_id: ClientId,
        _prefix: Vec<u8>,
        _path: String,
        _value: Vec<u8>,
        _proof_height: Height,
        _proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

    fn verify_non_membership(
        &self,
        _ctx: &dyn ClientReader,
        _client_id: ClientId,
        _prefix: Vec<u8>,
        _path: String,
        _proof_height: Height,
        _proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }
}

pub fn register_implementations(registry: &mut LightClientRegistry) {
    registry
        .put(
            LCP_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(LCPLightClient),
        )
        .unwrap()
}

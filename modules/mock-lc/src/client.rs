use crate::context::IBCContext;
use crate::errors::Error;
use crate::header::Header;
use crate::prelude::*;
use crate::state::{gen_state_id, ClientState, ConsensusState};
use commitments::{gen_state_id_from_any, UpdateClientCommitment};
use ibc::core::ics02_client::client_state::{
    downcast_client_state, ClientState as Ics02ClientState, UpdatedState,
};
use ibc::core::ics02_client::consensus_state::downcast_consensus_state;
use ibc::core::ics02_client::error::ClientError as ICS02Error;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::core::ics24_host::identifier::ClientId;
use ibc::mock::client_state::{client_type, MockClientState, MOCK_CLIENT_STATE_TYPE_URL};
use ibc::mock::consensus_state::MockConsensusState;
use lcp_types::{Any, Height, Time};
use light_client::{
    ClientReader, CreateClientResult, Error as LightClientError, LightClient,
    StateVerificationResult, UpdateClientResult,
};
use light_client_registry::LightClientRegistry;
use validation_context::ValidationParams;

#[derive(Default)]
pub struct MockLightClient;

impl LightClient for MockLightClient {
    fn client_type(&self) -> String {
        client_type().as_str().to_string()
    }

    fn latest_height(
        &self,
        ctx: &dyn ClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state: ClientState = ctx
            .client_state(client_id)
            .map_err(Error::ics02)?
            .try_into()?;
        Ok(client_state.latest_height().into())
    }

    fn create_client(
        &self,
        _: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let state_id = gen_state_id_from_any(&any_client_state, &any_consensus_state)
            .map_err(Error::commitment)?;

        let client_state: ClientState = any_client_state.clone().try_into()?;
        let consensus_state: ConsensusState = any_consensus_state.try_into()?;
        let height = client_state.latest_height().into();
        let timestamp: Time = consensus_state.timestamp().into();
        Ok(CreateClientResult {
            height,
            commitment: UpdateClientCommitment {
                prev_state_id: None,
                new_state_id: state_id,
                new_state: Some(any_client_state.into()),
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
        let header = Header::try_from(any_header.clone())?;

        // Read client state from the host chain store.
        let client_state: ClientState = ctx
            .client_state(&client_id)
            .map_err(Error::ics02)?
            .try_into()?;

        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::ClientFrozen { client_id }).into());
        }

        let height = header.height().into();
        let header_timestamp = header.timestamp().into();

        let latest_height = client_state.latest_height();

        // Read consensus state from the host chain store.
        let latest_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, latest_height.into())
            .map_err(|_| {
                Error::ics02(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone(),
                    height: latest_height,
                })
            })?
            .try_into()?;

        // Use client_state to validate the new header against the latest consensus_state.
        // This function will return the new client_state (its latest_height changed) and a
        // consensus_state obtained from header. These will be later persisted by the keeper.
        let UpdatedState {
            client_state: new_client_state,
            consensus_state: new_consensus_state,
        } = client_state
            .check_header_and_update_state(&IBCContext::new(ctx), client_id, any_header.into())
            .map_err(|e| {
                Error::ics02(ICS02Error::HeaderVerificationFailure {
                    reason: e.to_string(),
                })
            })?;

        let new_client_state = ClientState(
            downcast_client_state::<MockClientState>(new_client_state.as_ref())
                .unwrap()
                .clone(),
        );
        let new_consensus_state = ConsensusState(
            downcast_consensus_state::<MockConsensusState>(new_consensus_state.as_ref())
                .unwrap()
                .clone(),
        );

        let prev_state_id = gen_state_id(client_state, latest_consensus_state)?;
        let new_state_id = gen_state_id(new_client_state.clone(), new_consensus_state.clone())?;
        let new_any_client_state = Any::try_from(new_client_state).unwrap();

        Ok(UpdateClientResult {
            new_any_client_state: new_any_client_state.clone(),
            new_any_consensus_state: Any::try_from(new_consensus_state).unwrap(),
            height,
            commitment: UpdateClientCommitment {
                prev_state_id: Some(prev_state_id),
                new_state_id,
                new_state: new_any_client_state.into(),
                prev_height: Some(latest_height.into()),
                new_height: height,
                timestamp: header_timestamp,
                validation_params: ValidationParams::Empty,
            },
            prove: true,
        })
    }

    #[allow(unused_variables)]
    fn verify_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

    #[allow(unused_variables)]
    fn verify_non_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }
}

pub fn register_implementations(registry: &mut dyn LightClientRegistry) {
    registry
        .put_light_client(
            MOCK_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(MockLightClient),
        )
        .unwrap()
}

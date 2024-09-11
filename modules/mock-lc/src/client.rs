use crate::errors::Error;
use crate::message::{ClientMessage, Header, Misbehaviour};
use crate::prelude::*;
use crate::state::{gen_state_id, ClientState, ConsensusState};
use ibc::core::ics02_client::client_state::{
    downcast_client_state, ClientState as Ics02ClientState, UpdatedState,
};
use ibc::core::ics02_client::consensus_state::downcast_consensus_state;
use ibc::core::ics02_client::error::ClientError as ICS02Error;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::mock::client_state::{client_type, MockClientState, MOCK_CLIENT_STATE_TYPE_URL};
use ibc::mock::consensus_state::MockConsensusState;
use light_client::commitments::{
    gen_state_id_from_any, EmittedState, MisbehaviourProxyMessage, PrevState,
    UpdateStateProxyMessage, ValidationContext,
};
use light_client::types::{Any, ClientId, Height, Time};
use light_client::{
    ibc::IBCContext, CreateClientResult, Error as LightClientError, HostClientReader, LightClient,
    LightClientRegistry, MisbehaviourData, UpdateClientResult, UpdateStateData,
    VerifyMembershipResult, VerifyNonMembershipResult,
};

#[derive(Default)]
pub struct MockLightClient;

impl LightClient for MockLightClient {
    fn client_type(&self) -> String {
        client_type().as_str().to_string()
    }

    fn latest_height(
        &self,
        ctx: &dyn HostClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state: ClientState = ctx.client_state(client_id)?.try_into()?;
        Ok(client_state.latest_height().into())
    }

    fn create_client(
        &self,
        _: &dyn HostClientReader,
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
            message: UpdateStateProxyMessage {
                prev_height: None,
                prev_state_id: None,
                post_state_id: state_id,
                post_height: height,
                timestamp,
                context: ValidationContext::Empty,
                emitted_states: vec![EmittedState(height, any_client_state)],
            }
            .into(),
            prove: false,
        })
    }

    fn update_client(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        any_client_message: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let client_message = ClientMessage::try_from(any_client_message)?;
        match client_message {
            ClientMessage::Header(header) => Ok(self.update_state(ctx, client_id, header)?.into()),
            ClientMessage::Misbehaviour(misbehaviour) => Ok(self
                .submit_misbehaviour(ctx, client_id, misbehaviour)?
                .into()),
        }
    }

    #[allow(unused_variables)]
    fn verify_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<VerifyMembershipResult, LightClientError> {
        todo!()
    }

    #[allow(unused_variables)]
    fn verify_non_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<VerifyNonMembershipResult, LightClientError> {
        todo!()
    }
}

impl MockLightClient {
    fn update_state(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        header: Header,
    ) -> Result<UpdateStateData, LightClientError> {
        // Read client state from the host chain store.
        let client_state: ClientState = ctx.client_state(&client_id)?.try_into()?;

        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::ClientFrozen {
                client_id: client_id.into(),
            })
            .into());
        }

        let height = header.height().into();
        let header_timestamp = header.timestamp().into();

        let latest_height = client_state.latest_height();

        // Read consensus state from the host chain store.
        let latest_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, &latest_height.into())
            .map_err(|_| {
                Error::ics02(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone().into(),
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
            .check_header_and_update_state(
                &IBCContext::<MockClientState, MockConsensusState>::new(ctx),
                client_id.into(),
                Any::from(header).into(),
            )
            .map_err(|e| {
                Error::ics02(ICS02Error::HeaderVerificationFailure {
                    reason: e.to_string(),
                })
            })?;

        let new_client_state = ClientState(
            *downcast_client_state::<MockClientState>(new_client_state.as_ref()).unwrap(),
        );
        let new_consensus_state = ConsensusState(
            downcast_consensus_state::<MockConsensusState>(new_consensus_state.as_ref())
                .unwrap()
                .clone(),
        );

        let prev_state_id = gen_state_id(client_state, latest_consensus_state)?;
        let post_state_id = gen_state_id(new_client_state.clone(), new_consensus_state.clone())?;
        let new_any_client_state = Any::from(new_client_state);

        Ok(UpdateStateData {
            new_any_client_state: new_any_client_state.clone(),
            new_any_consensus_state: Any::from(new_consensus_state),
            height,
            message: UpdateStateProxyMessage {
                prev_height: Some(latest_height.into()),
                prev_state_id: Some(prev_state_id),
                post_height: height,
                post_state_id,
                timestamp: header_timestamp,
                context: ValidationContext::Empty,
                emitted_states: vec![EmittedState(height, new_any_client_state)],
            },
            prove: true,
        })
    }

    fn submit_misbehaviour(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        misbehaviour: Misbehaviour,
    ) -> Result<MisbehaviourData, LightClientError> {
        // WARNING: misbehaviour of ibc-rs's mock client has a bug where the client_id is set to `07-tendermint-0` when decoding from `Any`.
        // assert_eq!(client_id, misbehaviour.client_id());

        // Read client state from the host chain store.
        let client_state: ClientState = ctx.client_state(&client_id)?.try_into()?;
        let latest_height = client_state.latest_height();
        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::ClientFrozen {
                client_id: client_id.into(),
            })
            .into());
        }

        // Read consensus state from the host chain store.
        let latest_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, &latest_height.into())
            .map_err(|_| {
                Error::ics02(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone().into(),
                    height: latest_height,
                })
            })?
            .try_into()?;

        let new_client_state = client_state
            .check_misbehaviour_and_update_state(
                &IBCContext::<MockClientState, MockConsensusState>::new(ctx),
                client_id.into(),
                Any::from(misbehaviour.clone()).into(),
            )
            .unwrap();

        let new_client_state = ClientState(
            *downcast_client_state::<MockClientState>(new_client_state.as_ref()).unwrap(),
        );

        Ok(MisbehaviourData {
            new_any_client_state: Any::from(new_client_state),
            message: MisbehaviourProxyMessage {
                prev_states: vec![PrevState {
                    height: latest_height.into(),
                    state_id: gen_state_id(client_state, latest_consensus_state)?,
                }],
                context: ValidationContext::Empty,
                client_message: Any::from(misbehaviour),
            },
        })
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

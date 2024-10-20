use crate::context::MockClientIBCContext;
use crate::ibc::client_state::{MockClientState, MOCK_CLIENT_STATE_TYPE_URL, MOCK_CLIENT_TYPE};
use crate::ibc::consensus_state::MockConsensusState;
use crate::prelude::*;
use ibc_core_host_types::path::PathBytes;
use ibc_primitives::proto::Any as IBCAny;
use light_client::commitments::{gen_state_id_from_any, StateID};
use light_client::ibc::CreateExecutionResult;
use light_client::{
    commitments::EmittedState,
    ibc::{IBCHandler, UpdateExecutionResult},
    types::{Any, ClientId, Height},
    CreateClientResult, Error as LightClientError, HostClientReader, LightClient,
    LightClientRegistry, MisbehaviourData, UpdateClientResult, UpdateStateData,
    VerifyMembershipResult, VerifyNonMembershipResult,
};

#[derive(Default)]
pub struct MockLightClient;

// TODO error handling
impl LightClient for MockLightClient {
    fn client_type(&self) -> String {
        MOCK_CLIENT_TYPE.to_string()
    }

    fn latest_height(
        &self,
        ctx: &dyn HostClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state: MockClientState = IBCAny::from(ctx.client_state(client_id)?)
            .try_into()
            .unwrap();
        Ok(client_state.latest_height().into())
    }

    fn create_client(
        &self,
        ctx: &dyn HostClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let mut ibc_ctx = MockClientIBCContext::new(ctx);
        let CreateExecutionResult(latest_height) = IBCHandler::create_client(
            ctx,
            &mut ibc_ctx,
            ctx.host_timestamp().into(),
            any_client_state.into(),
            any_consensus_state.into(),
        )
        .unwrap();

        let (_, pmsg) =
            ibc_ctx.gen_initialize_state_proxy_message(|client_state, consensus_state| {
                gen_state_id(*client_state, consensus_state.clone())
            });
        Ok(CreateClientResult {
            height: latest_height.into(),
            message: pmsg.into(),
            prove: false,
        })
    }

    fn update_client(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        any_client_message: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let ibc_client_message: IBCAny = any_client_message.clone().into();
        let mut ibc_ctx = MockClientIBCContext::new(ctx);
        let ibc_client_id = client_id.into();

        match IBCHandler::update_client(&mut ibc_ctx, &ibc_client_id, ibc_client_message.clone())
            .unwrap()
        {
            UpdateExecutionResult::Success(_) => {
                let ((client_state, consensus_state), mut pmsg) = ibc_ctx
                    .gen_update_state_proxy_message(
                        &ibc_client_id,
                        |client_state, consensus_state| {
                            gen_state_id(*client_state, consensus_state.clone())
                        },
                    );
                pmsg.emitted_states
                    .push(EmittedState(pmsg.post_height, any_client_message));
                Ok(UpdateClientResult::UpdateState(UpdateStateData {
                    new_any_client_state: IBCAny::from(client_state).into(),
                    new_any_consensus_state: IBCAny::from(consensus_state).into(),
                    height: client_state.latest_height().into(),
                    message: pmsg,
                    prove: true,
                }))
            }
            UpdateExecutionResult::Misbehaviour => {
                let pmsg = ibc_ctx.gen_misbehaviour_proxy_message(
                    &ibc_client_id,
                    ibc_client_message,
                    |client_state, consensus_state| {
                        gen_state_id(*client_state, consensus_state.clone())
                    },
                );
                Ok(UpdateClientResult::Misbehaviour(MisbehaviourData {
                    new_any_client_state: any_client_message,
                    message: pmsg,
                }))
            }
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
        let ibc_ctx = MockClientIBCContext::new(ctx);
        IBCHandler::verify_membership(
            &ibc_ctx,
            &client_id.clone().into(),
            &prefix.clone().into(),
            proof_height.try_into().unwrap(),
            &proof.try_into().unwrap(),
            PathBytes::from_bytes(path.as_bytes()),
            value.clone(),
        )
        .unwrap();
        let message = ibc_ctx.gen_membership_proxy_message(
            &client_id.into(),
            prefix.into(),
            path,
            Some(value),
            |client_state, consensus_state| gen_state_id(*client_state, consensus_state.clone()),
        );
        Ok(VerifyMembershipResult { message })
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
        let ibc_ctx = MockClientIBCContext::new(ctx);
        IBCHandler::verify_non_membership(
            &ibc_ctx,
            &client_id.clone().into(),
            &prefix.clone().into(),
            proof_height.try_into().unwrap(),
            &proof.try_into().unwrap(),
            PathBytes::from_bytes(path.as_bytes()),
        )
        .unwrap();
        let message = ibc_ctx.gen_membership_proxy_message(
            &client_id.into(),
            prefix.into(),
            path,
            None,
            |client_state, consensus_state| gen_state_id(*client_state, consensus_state.clone()),
        );
        Ok(VerifyNonMembershipResult { message })
    }
}

pub fn gen_state_id(client_state: MockClientState, consensus_state: MockConsensusState) -> StateID {
    // Safe to unwrap since `gen_state_id_from_any` never returns an error actually
    gen_state_id_from_any(
        &IBCAny::from(client_state).into(),
        &IBCAny::from(consensus_state).into(),
    )
    .unwrap()
}

pub fn register_implementations(registry: &mut dyn LightClientRegistry) {
    registry
        .put_light_client(
            MOCK_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(MockLightClient),
        )
        .unwrap()
}

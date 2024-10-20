use crate::context::TendermintIBCContext;
use crate::prelude::*;
use crate::state::gen_state_id;
use ibc_client_tendermint::client_state::ClientState as TendermintClientState;
use ibc_client_tendermint::types::TENDERMINT_CLIENT_STATE_TYPE_URL;
use ibc_client_tendermint::TENDERMINT_CLIENT_TYPE;
use ibc_core_host_types::path::PathBytes;
use ibc_primitives::proto::Any as IBCAny;
use light_client::{
    ibc::{CreateExecutionResult, IBCHandler, UpdateExecutionResult},
    types::{Any, ClientId, Height},
    CreateClientResult, Error as LightClientError, HostClientReader, LightClient,
    LightClientRegistry, MisbehaviourData, UpdateClientResult, UpdateStateData,
    VerifyMembershipResult, VerifyNonMembershipResult,
};

pub fn register_implementations(registry: &mut dyn LightClientRegistry) {
    registry
        .put_light_client(
            TENDERMINT_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(TendermintLightClient),
        )
        .unwrap()
}

#[derive(Default)]
pub struct TendermintLightClient;

impl LightClient for TendermintLightClient {
    fn client_type(&self) -> String {
        TENDERMINT_CLIENT_TYPE.to_string()
    }

    fn latest_height(
        &self,
        ctx: &dyn HostClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state: TendermintClientState = IBCAny::from(ctx.client_state(client_id)?)
            .try_into()
            .unwrap();
        Ok(client_state.inner().latest_height.into())
    }

    fn create_client(
        &self,
        ctx: &dyn HostClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let mut ibc_ctx = TendermintIBCContext::new(ctx);
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
                gen_state_id(client_state.clone(), consensus_state.clone())
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
        let mut ibc_ctx = TendermintIBCContext::new(ctx);
        let ibc_client_id = client_id.into();

        match IBCHandler::update_client(&mut ibc_ctx, &ibc_client_id, ibc_client_message.clone())
            .unwrap()
        {
            UpdateExecutionResult::Success(_) => {
                let ((client_state, consensus_state), pmsg) = ibc_ctx
                    .gen_update_state_proxy_message(
                        &ibc_client_id,
                        |client_state, consensus_state| {
                            gen_state_id(client_state.clone(), consensus_state.clone())
                        },
                    );
                Ok(UpdateClientResult::UpdateState(UpdateStateData {
                    new_any_client_state: IBCAny::from(client_state.clone()).into(),
                    new_any_consensus_state: IBCAny::from(consensus_state).into(),
                    height: client_state.inner().latest_height.into(),
                    message: pmsg,
                    prove: true,
                }))
            }
            UpdateExecutionResult::Misbehaviour => {
                let pmsg = ibc_ctx.gen_misbehaviour_proxy_message(
                    &ibc_client_id,
                    ibc_client_message,
                    |client_state, consensus_state| {
                        gen_state_id(client_state.clone(), consensus_state.clone())
                    },
                );
                Ok(UpdateClientResult::Misbehaviour(MisbehaviourData {
                    new_any_client_state: any_client_message,
                    message: pmsg,
                }))
            }
        }
    }

    fn verify_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: light_client::commitments::CommitmentPrefix,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<VerifyMembershipResult, LightClientError> {
        let ibc_ctx = TendermintIBCContext::new(ctx);
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
            |client_state, consensus_state| {
                gen_state_id(client_state.clone(), consensus_state.clone())
            },
        );
        Ok(VerifyMembershipResult { message })
    }

    fn verify_non_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: light_client::commitments::CommitmentPrefix,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<VerifyNonMembershipResult, LightClientError> {
        let ibc_ctx = TendermintIBCContext::new(ctx);
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
            |client_state, consensus_state| {
                gen_state_id(client_state.clone(), consensus_state.clone())
            },
        );
        Ok(VerifyNonMembershipResult { message })
    }
}

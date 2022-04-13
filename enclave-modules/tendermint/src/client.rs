use crate::errors::TendermintError as Error;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id, gen_state_id_from_any, StateCommitment, UpdateClientCommitment};
use enclave_light_client::LightClientError;
use enclave_light_client::{CreateClientResult, UpdateClientResult, VerifyClientResult};
use enclave_light_client::{LightClient, LightClientRegistry};
use ibc::core::ics02_client::client_consensus::{AnyConsensusState, ConsensusState};
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{AnyClientState, ClientState};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::context::ClientReader;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics03_connection::context::ConnectionReader;
use ibc::core::ics03_connection::error::Error as ICS03Error;
use ibc::core::ics23_commitment::commitment::{CommitmentPrefix, CommitmentProofBytes};
use ibc::core::ics24_host::identifier::ClientId;
use ibc::core::ics24_host::path::ClientStatePath;
use ibc::core::ics24_host::Path;
use ibc::Height;
use log::*;
use prost_types::Any;
use serde_json::Value;
use std::boxed::Box;
use std::string::{String, ToString};
use std::vec::Vec;
use tendermint_light_client_verifier::options::Options;
use tendermint_proto::Protobuf;
use validation_context::tendermint::{TendermintValidationParams, TendermintValidationPredicate};
use validation_context::ValidationParams;

#[derive(Default)]
pub struct TendermintLightClient;

impl LightClient for TendermintLightClient {
    fn create_client(
        &self,
        ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let client_state = match AnyClientState::try_from(any_client_state.clone()) {
            Ok(AnyClientState::Tendermint(client_state)) => {
                AnyClientState::Tendermint(client_state)
            }
            #[cfg(any(test, feature = "mocks"))]
            Ok(_) => return Err(Error::UnexpectedClientTypeError(any_client_state.type_url).into()),
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };
        let consensus_state = match AnyConsensusState::try_from(any_consensus_state.clone()) {
            Ok(AnyConsensusState::Tendermint(consensus_state)) => {
                AnyConsensusState::Tendermint(consensus_state)
            }
            #[cfg(any(test, feature = "mocks"))]
            Ok(_) => {
                return Err(Error::UnexpectedClientTypeError(any_consensus_state.type_url).into())
            }
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };

        let client_id = gen_client_id(&any_client_state, &any_consensus_state)?;
        let height = client_state.latest_height();
        let timestamp = consensus_state.timestamp();

        let state_id = gen_state_id_from_any(&any_client_state, &any_consensus_state)
            .map_err(|e| Error::OtherError(e).into())?;

        Ok(CreateClientResult {
            client_id: client_id.clone(),
            client_type: ClientType::Tendermint.as_str().to_owned(),
            any_client_state,
            any_consensus_state,
            height,
            timestamp,
            commitment: UpdateClientCommitment {
                client_id,
                prev_state_id: None,
                new_state_id: state_id,
                prev_height: None,
                new_height: height,
                timestamp: timestamp.nanoseconds(),
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
        let (header, trusted_height) = match AnyHeader::try_from(any_header) {
            Ok(AnyHeader::Tendermint(header)) => {
                let trusted_height = header.trusted_height;
                (AnyHeader::Tendermint(header), trusted_height)
            }
            #[cfg(any(test, feature = "mocks"))]
            Ok(_) => return Err(Error::UnexpectedClientTypeError(any_client_state.type_url).into()),
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };

        // Read client type from the host chain store. The client should already exist.
        let client_type = ctx
            .client_type(&client_id)
            .map_err(|e| Error::ICS02Error(e).into())?;

        let client_def = AnyClient::from_client_type(client_type);

        // Read client state from the host chain store.
        let client_state = ctx
            .client_state(&client_id)
            .map_err(|e| Error::ICS02Error(e).into())?;

        if client_state.is_frozen() {
            return Err(Error::ICS02Error(ICS02Error::client_frozen(client_id)).into());
        }

        // Read consensus state from the host chain store.
        let latest_consensus_state = ctx
            .consensus_state(&client_id, client_state.latest_height())
            .map_err(|_| {
                Error::ICS02Error(ICS02Error::consensus_state_not_found(
                    client_id.clone(),
                    client_state.latest_height(),
                ))
                .into()
            })?;

        debug!("latest consensus state: {:?}", latest_consensus_state);

        let now = ctx.host_timestamp();
        let duration = now
            .duration_since(&latest_consensus_state.timestamp())
            .ok_or_else(|| {
                Error::ICS02Error(ICS02Error::invalid_consensus_state_timestamp(
                    latest_consensus_state.timestamp(),
                    now,
                ))
                .into()
            })?;

        if client_state.expired(duration) {
            return Err(
                Error::ICS02Error(ICS02Error::header_not_within_trust_period(
                    latest_consensus_state.timestamp(),
                    header.timestamp(),
                ))
                .into(),
            );
        }

        let height = header.height();
        let timestamp = header.timestamp();

        let trusted_consensus_state =
            ctx.consensus_state(&client_id, trusted_height)
                .map_err(|_| {
                    Error::ICS02Error(ICS02Error::consensus_state_not_found(
                        client_id.clone(),
                        trusted_height,
                    ))
                    .into()
                })?;

        // Use client_state to validate the new header against the latest consensus_state.
        // This function will return the new client_state (its latest_height changed) and a
        // consensus_state obtained from header. These will be later persisted by the keeper.
        let (new_client_state, new_consensus_state) = client_def
            .check_header_and_update_state(ctx, client_id.clone(), client_state.clone(), header)
            .map_err(|e| {
                Error::ICS02Error(ICS02Error::header_verification_failure(e.to_string())).into()
            })?;

        let trusted_consensus_state_timestamp = trusted_consensus_state.timestamp().nanoseconds();
        let options = match client_state {
            AnyClientState::Tendermint(ref client_state) => Options {
                trust_threshold: client_state
                    .trust_level
                    .try_into()
                    .map_err(|e| Error::ICS02Error(e).into())?,
                trusting_period: client_state.trusting_period,
                clock_drift: client_state.max_clock_drift,
            },
            _ => unreachable!(),
        };

        let prev_state_id = gen_state_id(client_state, trusted_consensus_state)
            .map_err(|e| Error::OtherError(e).into())?;
        let new_state_id = gen_state_id(new_client_state.clone(), new_consensus_state.clone())
            .map_err(|e| Error::OtherError(e).into())?;

        Ok(UpdateClientResult {
            client_id: client_id.clone(),
            new_any_client_state: Any::from(new_client_state),
            new_any_consensus_state: Any::from(new_consensus_state),
            height,
            timestamp,
            commitment: UpdateClientCommitment {
                client_id,
                prev_state_id: Some(prev_state_id),
                new_state_id,
                prev_height: Some(trusted_height),
                new_height: height,
                timestamp: timestamp.nanoseconds(),
                validation_params: ValidationParams::Tendermint(TendermintValidationParams {
                    options,
                    header_timestamp: timestamp.nanoseconds(),
                    trusted_consensus_state_timestamp,
                }),
            },
        })
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
    ) -> Result<VerifyClientResult, LightClientError> {
        // Read client state from the host chain store.
        let client_state = ctx
            .client_state(&client_id)
            .map_err(|e| Error::ICS03Error(e).into())?;

        if client_state.is_frozen() {
            return Err(Error::ICS02Error(ICS02Error::client_frozen(client_id)).into());
        }

        let consensus_state = ctx
            .client_consensus_state(&client_id, proof_height)
            .map_err(|e| Error::ICS03Error(e).into())?;

        let client_def = AnyClient::from_client_type(client_state.client_type());

        let proof: CommitmentProofBytes = proof
            .try_into()
            .map_err(|e| Error::IBCProofError(e).into())?;

        let prefix: CommitmentPrefix = counterparty_prefix
            .try_into()
            .map_err(|e| Error::ICS23Error(e).into())?;

        // TODO replace the following verification logic with owned method
        let expected_client_state = AnyClientState::try_from(expected_client_state.clone())
            .map_err(|e| Error::ICS02Error(e).into())?;
        client_def
            .verify_client_full_state(
                &client_state,
                proof_height,
                &prefix,
                &proof,
                consensus_state.root(),
                &counterparty_client_id,
                &expected_client_state,
            )
            .map_err(|e| {
                Error::ICS03Error(ICS03Error::client_state_verification_failure(
                    client_id.clone(),
                    e,
                ))
                .into()
            })?;

        let sc = StateCommitment {
            path: Path::ClientState(ClientStatePath(client_id)),
            value: expected_client_state.encode_vec().unwrap(),
            height: proof_height,
            state_id: gen_state_id_from_any(&Any::from(client_state), &Any::from(consensus_state))
                .map_err(|e| Error::OtherError(e).into())?,
        };

        Ok(VerifyClientResult {
            state_commitment: sc,
        })
    }
}

pub fn register_implementations(registry: &mut LightClientRegistry) {
    registry
        .put(
            ClientType::Tendermint.as_str().to_string(),
            Box::new(TendermintLightClient),
        )
        .unwrap()
}

pub fn gen_client_id(
    any_client_state: &Any,
    any_consensus_state: &Any,
) -> Result<ClientId, LightClientError> {
    let state_id = gen_state_id_from_any(any_client_state, any_consensus_state)
        .map_err(LightClientError::OtherError)?;
    Ok(serde_json::from_value::<ClientId>(Value::String(state_id.to_string())).unwrap())
}

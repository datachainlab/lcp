use crate::errors::TendermintError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id, gen_state_id_from_any, StateCommitment, UpdateClientCommitment};
use ibc::clients::ics07_tendermint::client_state::ClientState as TendermintClientState;
use ibc::core::ics02_client::client_consensus::{AnyConsensusState, ConsensusState};
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{
    AnyClientState, ClientState, TENDERMINT_CLIENT_STATE_TYPE_URL,
};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::context::ClientReader as IBCClientReader;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics03_connection::error::Error as ICS03Error;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics04_channel::error::Error as ICS04Error;
use ibc::core::ics23_commitment::commitment::{CommitmentPrefix, CommitmentProofBytes};
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::core::ics24_host::path::{
    ChannelEndsPath, ClientConsensusStatePath, ClientStatePath, ConnectionsPath,
};
use ibc::core::ics24_host::Path;
use lcp_types::{Any, Height};
use light_client::{
    ClientReader, CreateClientResult, LightClient, LightClientError, LightClientRegistry,
    StateVerificationResult, UpdateClientResult,
};
use log::*;
use serde_json::Value;
use std::boxed::Box;
use std::string::ToString;
use std::vec::Vec;
use tendermint_light_client_verifier::options::Options;
use tendermint_light_client_verifier::types::TrustThreshold;
use tendermint_proto::Protobuf;
use validation_context::tendermint::{TendermintValidationOptions, TendermintValidationParams};
use validation_context::ValidationParams;

#[derive(Default)]
pub struct TendermintLightClient;

impl LightClient for TendermintLightClient {
    fn create_client(
        &self,
        _: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let (client_state, canonical_client_state) =
            match AnyClientState::try_from(any_client_state.clone()) {
                Ok(AnyClientState::Tendermint(client_state)) => (
                    AnyClientState::Tendermint(client_state.clone()),
                    Any::from(AnyClientState::Tendermint(canonicalize_state(client_state))),
                ),
                #[allow(unreachable_patterns)]
                Ok(s) => {
                    return Err(
                        Error::UnexpectedClientTypeError(s.client_type().to_string()).into(),
                    )
                }
                Err(e) => return Err(Error::ICS02Error(e).into()),
            };
        let consensus_state = match AnyConsensusState::try_from(any_consensus_state.clone()) {
            Ok(AnyConsensusState::Tendermint(consensus_state)) => {
                AnyConsensusState::Tendermint(consensus_state)
            }
            #[allow(unreachable_patterns)]
            Ok(s) => {
                return Err(Error::UnexpectedClientTypeError(s.client_type().to_string()).into())
            }
            Err(e) => return Err(Error::ICS02Error(e).into()),
        };

        let client_id = gen_client_id(&canonical_client_state, &any_consensus_state)?;
        let state_id = gen_state_id_from_any(&canonical_client_state, &any_consensus_state)
            .map_err(Error::OtherError)?;

        let height = client_state.latest_height().into();
        let timestamp = consensus_state.timestamp();

        Ok(CreateClientResult {
            client_id: client_id.clone(),
            client_type: ClientType::Tendermint.as_str().to_owned(),
            any_client_state: any_client_state.clone(),
            any_consensus_state,
            height,
            timestamp,
            commitment: UpdateClientCommitment {
                client_id,
                prev_state_id: None,
                new_state_id: state_id,
                new_state: Some(any_client_state.into()),
                prev_height: None,
                new_height: height,
                timestamp: timestamp
                    .into_datetime()
                    .unwrap()
                    .unix_timestamp_nanos()
                    .try_into()
                    .unwrap(),
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
        let (trusted_height, header) = match AnyHeader::try_from(any_header) {
            Ok(AnyHeader::Tendermint(header)) => {
                (header.trusted_height, AnyHeader::Tendermint(header))
            }
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

        let canonical_client_state =
            AnyClientState::Tendermint(canonicalize_state_from_any(client_state.clone()));

        // Read consensus state from the host chain store.
        let latest_consensus_state = ctx
            .consensus_state(&client_id, client_state.latest_height())
            .map_err(|_| {
                Error::ICS02Error(ICS02Error::consensus_state_not_found(
                    client_id.clone(),
                    client_state.latest_height(),
                ))
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

        let height = header.height().into();
        let header_timestamp = header.timestamp();

        let trusted_consensus_state =
            ctx.consensus_state(&client_id, trusted_height)
                .map_err(|_| {
                    Error::ICS02Error(ICS02Error::consensus_state_not_found(
                        client_id.clone(),
                        trusted_height,
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
        let new_canonical_client_state =
            AnyClientState::Tendermint(canonicalize_state_from_any(new_client_state.clone()));

        let trusted_consensus_state_timestamp = trusted_consensus_state
            .timestamp()
            .into_datetime()
            .unwrap()
            .unix_timestamp_nanos()
            .try_into()
            .unwrap();
        let options = match client_state {
            AnyClientState::Tendermint(ref client_state) => Options {
                trust_threshold: TrustThreshold::new(
                    client_state.trust_level.numerator(),
                    client_state.trust_level.denominator(),
                )
                .unwrap(),
                trusting_period: client_state.trusting_period,
                clock_drift: client_state.max_clock_drift,
            },
            _ => unreachable!(),
        };

        let prev_state_id = gen_state_id(canonical_client_state, trusted_consensus_state)
            .map_err(Error::OtherError)?;
        let new_state_id = gen_state_id(new_canonical_client_state, new_consensus_state.clone())
            .map_err(Error::OtherError)?;
        let header_timestamp_nanos = header_timestamp
            .into_datetime()
            .unwrap()
            .unix_timestamp_nanos()
            .try_into()
            .unwrap();
        Ok(UpdateClientResult {
            client_id: client_id.clone(),
            new_any_client_state: new_client_state.into(),
            new_any_consensus_state: new_consensus_state.into(),
            height,
            timestamp: header_timestamp,
            commitment: UpdateClientCommitment {
                client_id,
                prev_state_id: Some(prev_state_id),
                new_state_id,
                new_state: None,
                prev_height: Some(trusted_height.into()),
                new_height: height,
                timestamp: header_timestamp_nanos,
                validation_params: ValidationParams::Tendermint(TendermintValidationParams {
                    options: TendermintValidationOptions {
                        trusting_period: options.trusting_period,
                        clock_drift: options.clock_drift,
                    },
                    untrusted_header_timestamp: header_timestamp_nanos,
                    trusted_consensus_state_timestamp,
                }),
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
        let (client_def, client_state, consensus_state, prefix, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            counterparty_prefix,
            proof_height,
            proof,
        )?;

        // TODO replace the following verification logic with owned method
        let expected_client_state =
            AnyClientState::try_from(expected_client_state.clone()).map_err(Error::ICS02Error)?;
        client_def
            .verify_client_full_state(
                &client_state,
                proof_height.try_into().map_err(Error::ICS02Error)?,
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
            })?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment {
                path: Path::ClientState(ClientStatePath(client_id)),
                value: expected_client_state.encode_vec().unwrap(),
                height: proof_height,
                state_id: gen_state_id(client_state, consensus_state).map_err(Error::OtherError)?,
            },
        })
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
        let (client_def, client_state, consensus_state, prefix, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            counterparty_prefix,
            proof_height,
            proof,
        )?;

        let expected_client_consensus_state =
            AnyConsensusState::try_from(expected_client_consensus_state)
                .map_err(Error::ICS02Error)?;
        let counterparty_consensus_height = counterparty_consensus_height
            .try_into()
            .map_err(Error::ICS02Error)?;

        client_def
            .verify_client_consensus_state(
                &client_state,
                proof_height.try_into().map_err(Error::ICS02Error)?,
                &prefix,
                &proof,
                consensus_state.root(),
                &counterparty_client_id,
                counterparty_consensus_height,
                &expected_client_consensus_state,
            )
            .map_err(|e| {
                Error::ICS03Error(ICS03Error::consensus_state_verification_failure(
                    counterparty_consensus_height,
                    e,
                ))
            })?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment {
                path: Path::ClientConsensusState(ClientConsensusStatePath {
                    client_id,
                    epoch: counterparty_consensus_height.revision_number(),
                    height: counterparty_consensus_height.revision_height(),
                }),
                value: expected_client_consensus_state.encode_vec().unwrap(),
                height: proof_height,
                state_id: gen_state_id(client_state, consensus_state).map_err(Error::OtherError)?,
            },
        })
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
        let (client_def, client_state, consensus_state, prefix, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            counterparty_prefix,
            proof_height,
            proof,
        )?;

        client_def
            .verify_connection_state(
                &client_state,
                proof_height.try_into().map_err(Error::ICS02Error)?,
                &prefix,
                &proof,
                consensus_state.root(),
                &counterparty_connection_id,
                &expected_connection_state,
            )
            .map_err(|e| Error::ICS03Error(ICS03Error::verify_connection_state(e)))?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment {
                path: Path::Connections(ConnectionsPath(counterparty_connection_id)),
                value: expected_connection_state.encode_vec().unwrap(),
                height: proof_height,
                state_id: gen_state_id(client_state, consensus_state).map_err(Error::OtherError)?,
            },
        })
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
        let (client_def, client_state, consensus_state, prefix, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            counterparty_prefix,
            proof_height,
            proof,
        )?;

        client_def
            .verify_channel_state(
                &client_state,
                proof_height.try_into().map_err(Error::ICS02Error)?,
                &prefix,
                &proof,
                consensus_state.root(),
                &counterparty_port_id,
                &counterparty_channel_id,
                &expected_channel_state,
            )
            .map_err(|e| Error::ICS04Error(ICS04Error::verify_channel_failed(e)))?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment {
                path: Path::ChannelEnds(ChannelEndsPath(
                    counterparty_port_id,
                    counterparty_channel_id,
                )),
                value: expected_channel_state.encode_vec().unwrap(),
                height: proof_height,
                state_id: gen_state_id(client_state, consensus_state).map_err(Error::OtherError)?,
            },
        })
    }
}

impl TendermintLightClient {
    fn validate_args(
        ctx: &dyn IBCClientReader,
        client_id: ClientId,
        counterparty_prefix: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<
        (
            AnyClient,
            AnyClientState,
            AnyConsensusState,
            CommitmentPrefix,
            CommitmentProofBytes,
        ),
        LightClientError,
    > {
        let client_state = ctx.client_state(&client_id).map_err(Error::ICS02Error)?;

        if client_state.is_frozen() {
            return Err(Error::ICS02Error(ICS02Error::client_frozen(client_id)).into());
        }

        let consensus_state = ctx
            .consensus_state(
                &client_id,
                proof_height.try_into().map_err(Error::ICS02Error)?,
            )
            .map_err(Error::ICS02Error)?;

        let client_def = AnyClient::from_client_type(client_state.client_type());

        let proof: CommitmentProofBytes = proof.try_into().map_err(Error::IBCProofError)?;

        let prefix: CommitmentPrefix = counterparty_prefix.try_into().map_err(Error::ICS23Error)?;

        Ok((client_def, client_state, consensus_state, prefix, proof))
    }
}

pub fn register_implementations(registry: &mut LightClientRegistry) {
    registry
        .put(
            TENDERMINT_CLIENT_STATE_TYPE_URL.to_string(),
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

// canonicalize_state canonicalizes some fields of specified client state
// target fields: latest_height, frozen_height
pub fn canonicalize_state(mut client_state: TendermintClientState) -> TendermintClientState {
    client_state.latest_height = Height::zero().try_into().unwrap();
    client_state.frozen_height = None;
    client_state
}

// wrapper function for canonicalize_state
pub fn canonicalize_state_from_any(client_state: AnyClientState) -> TendermintClientState {
    #[allow(irrefutable_let_patterns)]
    if let AnyClientState::Tendermint(tm_client_state) = client_state {
        canonicalize_state(tm_client_state)
    } else {
        unreachable!()
    }
}

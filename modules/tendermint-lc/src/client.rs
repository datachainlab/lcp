use crate::errors::TendermintError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use alloc::borrow::ToOwned;
use commitments::{gen_state_id, gen_state_id_from_any, StateCommitment, UpdateClientCommitment};
use core::str::FromStr;
use crypto::Keccak256;
use ibc::clients::ics07_tendermint::client_state::ClientState as TendermintClientState;
use ibc::clients::ics07_tendermint::error::Error as TendermintError;
use ibc::core::ics02_client::client_consensus::{AnyConsensusState, ConsensusState};
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{
    AnyClientState, ClientState, TENDERMINT_CLIENT_STATE_TYPE_URL,
};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::context::ClientReader as IBCClientReader;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics03_connection::error::Error as ICS03Error;
use ibc::core::ics23_commitment::commitment::{
    CommitmentPrefix, CommitmentProofBytes, CommitmentRoot,
};
use ibc::core::ics23_commitment::merkle::{apply_prefix, MerkleProof};
use ibc::core::ics24_host::identifier::ClientId;
use ibc::core::ics24_host::Path;
use ibc_proto::ibc::core::commitment::v1::MerkleProof as RawMerkleProof;
use lcp_types::{Any, Height, Time};
use light_client::{
    ClientReader, CreateClientResult, LightClient, LightClientError, LightClientRegistry,
    StateVerificationResult, UpdateClientResult,
};
use log::*;
use std::boxed::Box;
use std::string::{String, ToString};
use std::vec::Vec;
use tendermint_light_client_verifier::options::Options;
use tendermint_light_client_verifier::types::TrustThreshold;
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

        let state_id = gen_state_id_from_any(&canonical_client_state, &any_consensus_state)
            .map_err(Error::OtherError)?;
        let consensus_state = match AnyConsensusState::try_from(any_consensus_state) {
            Ok(AnyConsensusState::Tendermint(consensus_state)) => {
                AnyConsensusState::Tendermint(consensus_state)
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

        let canonical_client_state = canonicalize_state_from_any(client_state.clone());

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
        let header_timestamp: Time = header.timestamp().into();

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
            .check_header_and_update_state(ctx, client_id, client_state.clone(), header)
            .map_err(|e| {
                Error::ICS02Error(ICS02Error::header_verification_failure(e.to_string()))
            })?;
        let new_canonical_client_state = canonicalize_state_from_any(new_client_state.clone());

        let trusted_consensus_state_timestamp: Time = trusted_consensus_state.timestamp().into();
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
        Ok(UpdateClientResult {
            new_any_client_state: new_client_state.into(),
            new_any_consensus_state: new_consensus_state.into(),
            height,
            commitment: UpdateClientCommitment {
                prev_state_id: Some(prev_state_id),
                new_state_id,
                new_state: None,
                prev_height: Some(trusted_height.into()),
                new_height: height,
                timestamp: header_timestamp,
                validation_params: ValidationParams::Tendermint(TendermintValidationParams {
                    options: TendermintValidationOptions {
                        trusting_period: options.trusting_period,
                        clock_drift: options.clock_drift,
                    },
                    untrusted_header_timestamp: header_timestamp,
                    trusted_consensus_state_timestamp,
                }),
            },
            prove: true,
        })
    }

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
        let (_, client_state, consensus_state, prefix, path, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            prefix,
            path,
            proof_height,
            proof,
        )?;

        let tm_client_state = match client_state {
            AnyClientState::Tendermint(ref cs) => cs,
            _ => unreachable!(),
        };

        tm_client_state
            .verify_height(proof_height.try_into().map_err(Error::ICS02Error)?)
            .map_err(|e| Error::ICS02Error(e.into()))?;

        verify_membership(
            tm_client_state,
            &prefix,
            &proof,
            consensus_state.root(),
            path.clone(),
            value.to_vec(),
        )
        .map_err(|e| {
            Error::ICS03Error(ICS03Error::client_state_verification_failure(
                client_id.clone(),
                e,
            ))
        })?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment::new(
                prefix,
                path,
                Some(value.keccak256()),
                proof_height,
                gen_state_id(canonicalize_state_from_any(client_state), consensus_state)
                    .map_err(Error::OtherError)?,
            ),
        })
    }

    fn verify_non_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        let (_, client_state, consensus_state, prefix, path, proof) = Self::validate_args(
            ctx.as_ibc_client_reader(),
            client_id.clone(),
            prefix,
            path,
            proof_height,
            proof,
        )?;

        let tm_client_state = match client_state {
            AnyClientState::Tendermint(ref cs) => cs,
            _ => unreachable!(),
        };

        tm_client_state
            .verify_height(proof_height.try_into().map_err(Error::ICS02Error)?)
            .map_err(|e| Error::ICS02Error(e.into()))?;

        verify_non_membership(
            tm_client_state,
            &prefix,
            &proof,
            consensus_state.root(),
            path.clone(),
        )
        .map_err(|e| {
            Error::ICS03Error(ICS03Error::client_state_verification_failure(
                client_id.clone(),
                e,
            ))
        })?;

        Ok(StateVerificationResult {
            state_commitment: StateCommitment::new(
                prefix,
                path,
                None,
                proof_height,
                gen_state_id(canonicalize_state_from_any(client_state), consensus_state)
                    .map_err(Error::OtherError)?,
            ),
        })
    }

    fn client_type(&self) -> String {
        ClientType::Tendermint.as_str().to_owned()
    }

    fn latest_height(
        &self,
        ctx: &dyn ClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let ctx = ctx.as_ibc_client_reader();
        let client_state = ctx.client_state(client_id).map_err(Error::ICS02Error)?;
        Ok(client_state.latest_height().into())
    }
}

impl TendermintLightClient {
    fn validate_args(
        ctx: &dyn IBCClientReader,
        client_id: ClientId,
        counterparty_prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<
        (
            AnyClient,
            AnyClientState,
            AnyConsensusState,
            CommitmentPrefix,
            Path,
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

        let path: Path = Path::from_str(&path).unwrap();

        Ok((
            client_def,
            client_state,
            consensus_state,
            prefix,
            path,
            proof,
        ))
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

// canonicalize_state canonicalizes some fields of specified client state
// target fields: latest_height, frozen_height
pub fn canonicalize_state(mut client_state: TendermintClientState) -> TendermintClientState {
    client_state.latest_height = Height::zero().try_into().unwrap();
    client_state.frozen_height = None;
    client_state
}

// wrapper function for canonicalize_state
pub fn canonicalize_state_from_any(client_state: AnyClientState) -> AnyClientState {
    #[allow(irrefutable_let_patterns)]
    if let AnyClientState::Tendermint(tm_client_state) = client_state {
        AnyClientState::Tendermint(canonicalize_state(tm_client_state))
    } else {
        unreachable!()
    }
}

fn verify_membership(
    client_state: &TendermintClientState,
    prefix: &CommitmentPrefix,
    proof: &CommitmentProofBytes,
    root: &CommitmentRoot,
    path: impl Into<Path>,
    value: Vec<u8>,
) -> Result<(), ICS02Error> {
    let merkle_path = apply_prefix(prefix, vec![path.into().to_string()]);
    let merkle_proof: MerkleProof = RawMerkleProof::try_from(proof.clone())
        .map_err(ICS02Error::invalid_commitment_proof)?
        .into();

    merkle_proof
        .verify_membership(
            &client_state.proof_specs,
            root.clone().into(),
            merkle_path,
            value,
            0,
        )
        .map_err(|e| ICS02Error::tendermint(TendermintError::ics23_error(e)))
}

fn verify_non_membership(
    client_state: &TendermintClientState,
    prefix: &CommitmentPrefix,
    proof: &CommitmentProofBytes,
    root: &CommitmentRoot,
    path: impl Into<Path>,
) -> Result<(), ICS02Error> {
    let merkle_path = apply_prefix(prefix, vec![path.into().to_string()]);
    let merkle_proof: MerkleProof = RawMerkleProof::try_from(proof.clone())
        .map_err(ICS02Error::invalid_commitment_proof)?
        .into();

    merkle_proof
        .verify_non_membership(&client_state.proof_specs, root.clone().into(), merkle_path)
        .map_err(|e| ICS02Error::tendermint(TendermintError::ics23_error(e)))
}

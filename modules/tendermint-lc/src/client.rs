use crate::errors::Error;
use crate::header::Header;
use crate::prelude::*;
use crate::state::{canonicalize_state, gen_state_id, ClientState, ConsensusState};
use core::str::FromStr;
use crypto::Keccak256;
use ibc::clients::ics07_tendermint::client_state::{
    ClientState as TendermintClientState, TENDERMINT_CLIENT_STATE_TYPE_URL,
};
use ibc::clients::ics07_tendermint::client_type;
use ibc::clients::ics07_tendermint::consensus_state::ConsensusState as TendermintConsensusState;
use ibc::core::ics02_client::client_state::{
    downcast_client_state, ClientState as Ics02ClientState, UpdatedState,
};
use ibc::core::ics02_client::consensus_state::{
    downcast_consensus_state, ConsensusState as Ics02ConsensusState,
};
use ibc::core::ics02_client::error::ClientError as ICS02Error;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::core::ics03_connection::error::ConnectionError as ICS03Error;
use ibc::core::ics23_commitment::commitment::{
    CommitmentPrefix as IBCCommitmentPrefix, CommitmentProofBytes as IBCCommitmentProofBytes,
    CommitmentRoot,
};
use ibc::core::ics23_commitment::merkle::{apply_prefix, MerkleProof};
use ibc::core::ics24_host::Path;
use lcp_proto::ibc::core::commitment::v1::MerkleProof as RawMerkleProof;
use light_client::commitments::{
    CommitmentContext, CommitmentPrefix, EmittedState, TrustingPeriodContext, UpdateClientMessage,
    VerifyMembershipMessage,
};
use light_client::types::{Any, ClientId, Height, Time};
use light_client::{
    ibc::IBCContext, CreateClientResult, Error as LightClientError, HostClientReader, LightClient,
    LightClientRegistry, StateVerificationResult, UpdateClientResult,
};
use log::*;

#[derive(Default)]
pub struct TendermintLightClient;

impl LightClient for TendermintLightClient {
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
        let client_state = ClientState::try_from(any_client_state.clone())?;
        let consensus_state = ConsensusState::try_from(any_consensus_state)?;
        let _ = client_state
            .initialise(consensus_state.0.clone().into())
            .map_err(Error::ics02)?;

        let canonical_client_state = canonicalize_state(&client_state);
        let height = client_state.latest_height().into();
        let timestamp: Time = consensus_state.timestamp.into();
        let state_id = gen_state_id(canonical_client_state, consensus_state)?;

        Ok(CreateClientResult {
            height,
            message: UpdateClientMessage {
                prev_height: None,
                prev_state_id: None,
                post_height: height,
                post_state_id: state_id,
                timestamp,
                context: CommitmentContext::Empty,
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
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let header = Header::try_from(any_header.clone())?;

        // Read client state from the host chain store.
        let client_state: ClientState = ctx.client_state(&client_id)?.try_into()?;

        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::ClientFrozen {
                client_id: client_id.into(),
            })
            .into());
        }

        // Read consensus state from the host chain store.
        let latest_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, &client_state.latest_height().into())
            .map_err(|_| {
                Error::ics02(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone().into(),
                    height: client_state.latest_height(),
                })
            })?
            .try_into()?;

        debug!("latest consensus state: {:?}", latest_consensus_state);

        let now = ctx.host_timestamp();
        let duration = now
            .duration_since(latest_consensus_state.timestamp().into_tm_time().unwrap())
            .map_err(|_| {
                Error::ics02(ICS02Error::InvalidConsensusStateTimestamp {
                    time1: latest_consensus_state.timestamp(),
                    time2: now.into(),
                })
            })?;

        if client_state.expired(duration) {
            return Err(Error::ics02(ICS02Error::HeaderNotWithinTrustPeriod {
                latest_time: latest_consensus_state.timestamp(),
                update_time: header.timestamp(),
            })
            .into());
        }

        let height = header.height().into();
        let header_timestamp: Time = header.timestamp().into();

        let trusted_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, &header.trusted_height.into())
            .map_err(|_| {
                Error::ics02(ICS02Error::ConsensusStateNotFound {
                    client_id: client_id.clone().into(),
                    height: header.trusted_height,
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
                &IBCContext::<TendermintClientState, TendermintConsensusState>::new(ctx),
                client_id.into(),
                any_header.into(),
            )
            .map_err(|e| {
                Error::ics02(ICS02Error::HeaderVerificationFailure {
                    reason: e.to_string(),
                })
            })?;

        let new_client_state = ClientState(
            downcast_client_state::<TendermintClientState>(new_client_state.as_ref())
                .unwrap()
                .clone(),
        );
        let new_consensus_state = ConsensusState(
            downcast_consensus_state::<TendermintConsensusState>(new_consensus_state.as_ref())
                .unwrap()
                .clone(),
        );

        let trusted_state_timestamp: Time = trusted_consensus_state.timestamp().into();
        let lc_opts = client_state.as_light_client_options().unwrap();

        let prev_state_id =
            gen_state_id(canonicalize_state(&client_state), trusted_consensus_state)?;
        let post_state_id = gen_state_id(
            canonicalize_state(&new_client_state),
            new_consensus_state.clone(),
        )?;
        Ok(UpdateClientResult {
            new_any_client_state: new_client_state.into(),
            new_any_consensus_state: new_consensus_state.into(),
            height,
            message: UpdateClientMessage {
                prev_height: Some(header.trusted_height.into()),
                prev_state_id: Some(prev_state_id),
                post_height: height,
                post_state_id,
                timestamp: header_timestamp,
                context: TrustingPeriodContext::new(
                    lc_opts.trusting_period,
                    lc_opts.clock_drift,
                    header_timestamp,
                    trusted_state_timestamp,
                )
                .into(),
                emitted_states: Default::default(),
            }
            .into(),
            prove: true,
        })
    }

    fn verify_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: CommitmentPrefix,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        let (client_state, consensus_state, prefix, path, proof) =
            Self::validate_args(ctx, client_id.clone(), prefix, path, proof_height, proof)?;

        client_state
            .verify_height(proof_height.try_into().map_err(Error::ics02)?)
            .map_err(|e| Error::ics02(e.into()))?;

        verify_membership(
            &client_state,
            &prefix,
            &proof,
            consensus_state.root(),
            path.clone(),
            value.to_vec(),
        )
        .map_err(|e| {
            Error::ics03(ICS03Error::ClientStateVerificationFailure {
                client_id: client_id.clone().into(),
                client_error: e,
            })
        })?;

        Ok(StateVerificationResult {
            message: VerifyMembershipMessage::new(
                prefix.into_vec(),
                path.to_string(),
                Some(value.keccak256()),
                proof_height,
                gen_state_id(canonicalize_state(&client_state), consensus_state)?,
            )
            .into(),
        })
    }

    fn verify_non_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        let (client_state, consensus_state, prefix, path, proof) =
            Self::validate_args(ctx, client_id.clone(), prefix, path, proof_height, proof)?;

        client_state
            .verify_height(proof_height.try_into().map_err(Error::ics02)?)
            .map_err(|e| Error::ics02(e.into()))?;

        verify_non_membership(
            &client_state,
            &prefix,
            &proof,
            consensus_state.root(),
            path.clone(),
        )
        .map_err(|e| {
            Error::ics03(ICS03Error::ClientStateVerificationFailure {
                client_id: client_id.clone().into(),
                client_error: e,
            })
        })?;

        Ok(StateVerificationResult {
            message: VerifyMembershipMessage::new(
                prefix.into_vec(),
                path.to_string(),
                None,
                proof_height,
                gen_state_id(canonicalize_state(&client_state), consensus_state)?,
            )
            .into(),
        })
    }
}

impl TendermintLightClient {
    fn validate_args(
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        counterparty_prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<
        (
            ClientState,
            ConsensusState,
            IBCCommitmentPrefix,
            Path,
            IBCCommitmentProofBytes,
        ),
        LightClientError,
    > {
        let client_state: ClientState = ctx.client_state(&client_id)?.try_into()?;

        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::ClientFrozen {
                client_id: client_id.into(),
            })
            .into());
        }

        let consensus_state: ConsensusState =
            ctx.consensus_state(&client_id, &proof_height)?.try_into()?;

        let proof: IBCCommitmentProofBytes = proof.try_into().map_err(Error::ics23)?;
        let prefix: IBCCommitmentPrefix = counterparty_prefix.try_into().map_err(Error::ics23)?;
        let path: Path = Path::from_str(&path).unwrap();
        Ok((client_state, consensus_state, prefix, path, proof))
    }
}

pub fn register_implementations(registry: &mut dyn LightClientRegistry) {
    registry
        .put_light_client(
            TENDERMINT_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(TendermintLightClient),
        )
        .unwrap()
}

fn verify_membership(
    client_state: &ClientState,
    prefix: &IBCCommitmentPrefix,
    proof: &IBCCommitmentProofBytes,
    root: &CommitmentRoot,
    path: impl Into<Path>,
    value: Vec<u8>,
) -> Result<(), ICS02Error> {
    let merkle_path = apply_prefix(prefix, vec![path.into().to_string()]);
    let merkle_proof: MerkleProof = RawMerkleProof::try_from(proof.clone())
        .map_err(ICS02Error::InvalidCommitmentProof)?
        .into();

    merkle_proof
        .verify_membership(
            &client_state.proof_specs,
            root.clone().into(),
            merkle_path,
            value,
            0,
        )
        .map_err(ICS02Error::Ics23Verification)
}

fn verify_non_membership(
    client_state: &ClientState,
    prefix: &IBCCommitmentPrefix,
    proof: &IBCCommitmentProofBytes,
    root: &CommitmentRoot,
    path: impl Into<Path>,
) -> Result<(), ICS02Error> {
    let merkle_path = apply_prefix(prefix, vec![path.into().to_string()]);
    let merkle_proof: MerkleProof = RawMerkleProof::try_from(proof.clone())
        .map_err(ICS02Error::InvalidCommitmentProof)?
        .into();

    merkle_proof
        .verify_non_membership(&client_state.proof_specs, root.clone().into(), merkle_path)
        .map_err(ICS02Error::Ics23Verification)
}

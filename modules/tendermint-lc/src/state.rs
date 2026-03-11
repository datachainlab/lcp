use crate::errors::Error;
use core::ops::Deref;
use ibc::clients::ics07_tendermint::{
    client_state::{
        AllowUpdate, ClientState as TendermintClientState, TENDERMINT_CLIENT_STATE_TYPE_URL,
    },
    consensus_state::{
        ConsensusState as TendermintConsensusState, TENDERMINT_CONSENSUS_STATE_TYPE_URL,
    },
};
use lcp_proto::google::protobuf::Any as ProtoAny;
use lcp_proto::ibc::lightclients::tendermint::v1::ClientState as RawTmClientState;
use light_client::commitments::{gen_state_id_from_any, StateID};
use light_client::types::{Any, Height};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientState(pub(crate) TendermintClientState);

impl Deref for ClientState {
    type Target = TendermintClientState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Any> for ClientState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == TENDERMINT_CLIENT_STATE_TYPE_URL {
            Ok(Self(
                TendermintClientState::try_from(any).map_err(Error::ics02)?,
            ))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

impl From<ClientState> for Any {
    fn from(value: ClientState) -> Self {
        ProtoAny::from(value.0).into()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConsensusState(pub(crate) TendermintConsensusState);

impl Deref for ConsensusState {
    type Target = TendermintConsensusState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == TENDERMINT_CONSENSUS_STATE_TYPE_URL {
            Ok(Self(
                TendermintConsensusState::try_from(any).map_err(Error::ics02)?,
            ))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

impl From<ConsensusState> for Any {
    fn from(value: ConsensusState) -> Self {
        ProtoAny::from(value.0).into()
    }
}

// canonicalize_state canonicalizes some fields of specified client state
// target fields: latest_height, frozen_height
pub fn canonicalize_state(client_state: &ClientState) -> ClientState {
    let raw_state: RawTmClientState = client_state.0.clone().into();
    let opt = client_state.as_light_client_options().unwrap();
    #[allow(deprecated)]
    let tm = TendermintClientState::new(
        client_state.chain_id.clone(),
        client_state.trust_level,
        client_state.trusting_period,
        client_state.unbonding_period,
        opt.clock_drift,
        Height::new(client_state.chain_id.version(), 0)
            .try_into()
            .unwrap(),
        client_state.proof_specs.clone(),
        client_state.upgrade_path.clone(),
        AllowUpdate {
            after_expiry: raw_state.allow_update_after_expiry,
            after_misbehaviour: raw_state.allow_update_after_misbehaviour,
        },
        None,
    )
    .unwrap();
    ClientState(tm)
}

pub fn gen_state_id(
    client_state: ClientState,
    consensus_state: ConsensusState,
) -> Result<StateID, Error> {
    Ok(gen_state_id_from_any(
        &client_state.into(),
        &consensus_state.into(),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use core::time::Duration;
    use ibc::core::ics02_client::trust_threshold::TrustThreshold;
    use ibc::core::ics23_commitment::specs::ProofSpecs;
    use ibc::core::ics24_host::identifier::ChainId;
    use lcp_proto::google::protobuf::Timestamp;
    use lcp_proto::ibc::core::commitment::v1::MerkleRoot;
    use lcp_proto::ibc::lightclients::tendermint::v1::ConsensusState as RawTmConsensusState;

    fn sample_client_state() -> ClientState {
        let chain_id = ChainId::new("test-chain".to_string(), 1);
        let latest_height = Height::new(1, 123).try_into().unwrap();
        let state = TendermintClientState::new(
            chain_id,
            TrustThreshold::ONE_THIRD,
            Duration::new(64000, 0),
            Duration::new(128000, 0),
            Duration::new(3, 0),
            latest_height,
            ProofSpecs::default(),
            vec!["upgrade".to_string(), "upgradedIBCState".to_string()],
            AllowUpdate {
                after_expiry: false,
                after_misbehaviour: false,
            },
            None,
        )
        .unwrap();
        ClientState(state)
    }

    fn sample_consensus_state() -> ConsensusState {
        let raw = RawTmConsensusState {
            timestamp: Some(Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            root: Some(MerkleRoot {
                hash: vec![0x11; 32],
            }),
            next_validators_hash: vec![0x22; 32],
        };
        ConsensusState(TendermintConsensusState::try_from(raw).unwrap())
    }

    #[test]
    fn state_id_preserved_across_query_create_roundtrip() {
        let planner_client_state = sample_client_state();
        let planner_consensus_state = sample_consensus_state();
        let planner_post_state_id = gen_state_id(
            canonicalize_state(&planner_client_state),
            planner_consensus_state.clone(),
        )
        .unwrap();

        // Simulate `query_client -> create_client` transfer: Any encode/decode round-trip.
        let queried_any_client: Any = planner_client_state.clone().into();
        let queried_any_consensus: Any = planner_consensus_state.clone().into();
        let worker_client_state = ClientState::try_from(queried_any_client.clone()).unwrap();
        let worker_consensus_state = ConsensusState::try_from(queried_any_consensus.clone()).unwrap();
        let worker_prev_state_id = gen_state_id(
            canonicalize_state(&worker_client_state),
            worker_consensus_state.clone(),
        )
        .unwrap();

        assert_eq!(planner_post_state_id, worker_prev_state_id);

        // Ensure the Any payload itself remains stable after round-trip.
        let roundtripped_any_client: Any = worker_client_state.into();
        let roundtripped_any_consensus: Any = worker_consensus_state.into();
        assert_eq!(queried_any_client, roundtripped_any_client);
        assert_eq!(queried_any_consensus, roundtripped_any_consensus);
    }
}

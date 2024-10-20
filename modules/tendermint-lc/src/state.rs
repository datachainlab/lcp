use ibc_client_tendermint::client_state::ClientState as TendermintClientState;
use ibc_client_tendermint::consensus_state::ConsensusState as TendermintConsensusState;
use ibc_client_tendermint::types::ClientState;
use ibc_primitives::proto::Any as IBCAny;
use light_client::commitments::{gen_state_id_from_any, StateID};
use light_client::types::Height;

// canonicalize_state canonicalizes some fields of specified client state
// target fields: latest_height, frozen_height
pub fn canonicalize_state(client_state: &TendermintClientState) -> TendermintClientState {
    let inner = client_state.inner();
    let opts = inner.as_light_client_options().unwrap();
    let inner = ClientState::new(
        inner.chain_id.clone(),
        inner.trust_level,
        inner.trusting_period,
        inner.unbonding_period,
        opts.clock_drift,
        Height::new(inner.chain_id.revision_number(), 0)
            .try_into()
            .unwrap(),
        inner.proof_specs.clone(),
        inner.upgrade_path.clone(),
        inner.allow_update,
    )
    .unwrap();
    inner.into()
}

pub fn gen_state_id(
    client_state: TendermintClientState,
    consensus_state: TendermintConsensusState,
) -> StateID {
    gen_state_id_from_any(
        &IBCAny::from(canonicalize_state(&client_state)).into(),
        &IBCAny::from(consensus_state).into(),
    )
    .unwrap()
}

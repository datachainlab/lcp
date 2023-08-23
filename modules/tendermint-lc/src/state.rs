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
use lcp_proto::google::protobuf::Any as IBCAny;
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
        let any: IBCAny = value.into();
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
        IBCAny::from(value.0).into()
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
        let any: IBCAny = value.into();
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
        IBCAny::from(value.0).into()
    }
}

// canonicalize_state canonicalizes some fields of specified client state
// target fields: latest_height, frozen_height
pub fn canonicalize_state(client_state: &ClientState) -> ClientState {
    let raw_state: RawTmClientState = client_state.0.clone().try_into().unwrap();
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
        &client_state.try_into().unwrap(),
        &consensus_state.try_into().unwrap(),
    )?)
}

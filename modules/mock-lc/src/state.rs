use crate::errors::Error;
use core::ops::Deref;
use ibc::mock::client_state::{MockClientState, MOCK_CLIENT_STATE_TYPE_URL};
use ibc::mock::consensus_state::{MockConsensusState, MOCK_CONSENSUS_STATE_TYPE_URL};
use light_client::commitments::{gen_state_id_from_any, StateID};
use light_client::types::proto::google::protobuf::Any as IBCAny;
use light_client::types::Any;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientState(pub(crate) MockClientState);

impl Deref for ClientState {
    type Target = MockClientState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MockClientState> for ClientState {
    fn from(value: MockClientState) -> Self {
        Self(value)
    }
}

impl TryFrom<Any> for ClientState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: IBCAny = value.into();
        if any.type_url == MOCK_CLIENT_STATE_TYPE_URL {
            Ok(Self(MockClientState::try_from(any).map_err(Error::ics02)?))
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
pub struct ConsensusState(pub(crate) MockConsensusState);

impl Deref for ConsensusState {
    type Target = MockConsensusState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MockConsensusState> for ConsensusState {
    fn from(value: MockConsensusState) -> Self {
        Self(value)
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: IBCAny = value.into();
        if any.type_url == MOCK_CONSENSUS_STATE_TYPE_URL {
            Ok(Self(
                MockConsensusState::try_from(any).map_err(Error::ics02)?,
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

pub fn gen_state_id(
    client_state: ClientState,
    consensus_state: ConsensusState,
) -> Result<StateID, Error> {
    Ok(gen_state_id_from_any(
        &client_state.try_into().unwrap(),
        &consensus_state.try_into().unwrap(),
    )?)
}

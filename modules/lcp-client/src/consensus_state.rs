use crate::errors::Error;
use crate::prelude::*;
use lcp_proto::ibc::lightclients::lcp::v1::ConsensusState as RawConsensusState;
use lcp_proto::protobuf::Protobuf;
use light_client::commitments::StateID;
use light_client::types::{Any, Time};
use prost::Message;
use serde::{Deserialize, Serialize};

pub const LCP_CONSENSUS_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ConsensusState";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub state_id: StateID,
    pub timestamp: Time, // means upstream's timestamp
}

impl ConsensusState {
    pub fn is_empty(&self) -> bool {
        self.state_id == Default::default() && self.timestamp.as_unix_timestamp_nanos() == 0
    }
}

impl From<ConsensusState> for RawConsensusState {
    fn from(value: ConsensusState) -> Self {
        RawConsensusState {
            state_id: value.state_id.to_vec(),
            timestamp: value.timestamp.as_unix_timestamp_secs(),
        }
    }
}

impl TryFrom<RawConsensusState> for ConsensusState {
    type Error = Error;

    fn try_from(raw: RawConsensusState) -> Result<Self, Self::Error> {
        Ok(ConsensusState {
            state_id: raw.state_id.as_slice().try_into().unwrap(),
            timestamp: Time::from_unix_timestamp_nanos(
                (raw.timestamp as u128).checked_mul(1_000_000_000).unwrap(),
            )?,
        })
    }
}

impl Protobuf<Any> for ConsensusState {}

impl From<ConsensusState> for Any {
    fn from(value: ConsensusState) -> Self {
        let value =
            RawConsensusState::try_from(value).expect("encoding to `Any` from `ConsensusState`");
        Any::new(
            LCP_CONSENSUS_STATE_TYPE_URL.to_string(),
            value.encode_to_vec(),
        )
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_CONSENSUS_STATE_TYPE_URL => {
                ConsensusState::try_from(RawConsensusState::decode(&*raw.value).unwrap())
            }
            type_url => Err(Error::unexpected_client_type(type_url.to_owned())),
        }
    }
}

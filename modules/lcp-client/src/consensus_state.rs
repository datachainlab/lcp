use crate::errors::Error;
use crate::prelude::*;
use commitments::StateID;
use ibc_proto::protobuf::Protobuf;
use lcp_proto::ibc::lightclients::lcp::v1::ConsensusState as RawConsensusState;
use lcp_types::{Any, Time};
use prost::Message;
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};

pub const LCP_CONSENSUS_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ConsensusState";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub state_id: StateID,
    pub timestamp: Time, // means upstream's timestamp
}

impl ConsensusState {
    pub fn is_empty(&self) -> bool {
        self.state_id.is_zero()
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
            timestamp: Time::from_unix_timestamp_secs(raw.timestamp).unwrap(),
        })
    }
}

impl Protobuf<ProtoAny> for ConsensusState {}

impl From<ConsensusState> for ProtoAny {
    fn from(value: ConsensusState) -> Self {
        let value =
            RawConsensusState::try_from(value).expect("encoding to `Any` from `ConsensusState`");
        ProtoAny {
            type_url: LCP_CONSENSUS_STATE_TYPE_URL.to_string(),
            value: value.encode_to_vec(),
        }
    }
}

impl TryFrom<ProtoAny> for ConsensusState {
    type Error = Error;

    fn try_from(raw: ProtoAny) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_CONSENSUS_STATE_TYPE_URL => {
                ConsensusState::try_from(RawConsensusState::decode(&*raw.value).unwrap())
            }
            _ => Err(Error::unexpected_client_type(raw.type_url)),
        }
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = Error;

    fn try_from(any: Any) -> Result<Self, Self::Error> {
        TryFrom::<ProtoAny>::try_from(any.into())
    }
}

impl From<ConsensusState> for Any {
    fn from(value: ConsensusState) -> Self {
        ProtoAny::from(value).try_into().unwrap()
    }
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::StateID;
use core::convert::Infallible;
use ibc::{
    core::{
        ics02_client::{
            client_consensus::AnyConsensusState, client_type::ClientType, error::Error,
        },
        ics23_commitment::commitment::CommitmentRoot,
    },
    timestamp::Timestamp,
};
use prost_types::Any;
use serde::{Deserialize, Serialize};
use tendermint_proto::Protobuf;

pub const LCP_CONSENSUS_STATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.ConsensusState";

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub state_id: StateID,
    pub timestamp: u128, // means upstream's timestamp
}

impl ConsensusState {
    pub fn is_empty(&self) -> bool {
        self.state_id.is_zero() && self.timestamp == 0
    }

    pub fn get_timestamp(&self) -> Timestamp {
        Timestamp::from_nanoseconds(self.timestamp as u64).unwrap()
    }
}

impl Protobuf<Any> for ConsensusState {}

impl From<ConsensusState> for Any {
    fn from(value: ConsensusState) -> Self {
        Any {
            type_url: LCP_CONSENSUS_STATE_TYPE_URL.to_string(),
            value: value
                .encode_vec()
                .expect("encoding to `Any` from `ConsensusState`"),
        }
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            "" => Err(Error::empty_client_state_response()),
            LCP_CONSENSUS_STATE_TYPE_URL => {
                Ok(ConsensusState::decode_vec(&raw.value)
                    .map_err(Error::decode_raw_client_state)?)
            }
            _ => Err(Error::unknown_consensus_state_type(raw.type_url)),
        }
    }
}

impl ibc::core::ics02_client::client_consensus::ConsensusState for ConsensusState {
    type Error = Infallible;

    fn client_type(&self) -> ClientType {
        todo!()
    }

    fn root(&self) -> &CommitmentRoot {
        panic!("not supported")
    }

    fn wrap_any(self) -> AnyConsensusState {
        panic!("not supported")
    }
}

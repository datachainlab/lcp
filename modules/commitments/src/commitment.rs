#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::CommitmentError as Error;
use crate::StateID;
use core::str::FromStr;
use ibc::core::ics23_commitment::commitment::CommitmentPrefix;
use ibc::core::ics24_host::Path;
use lcp_types::{Any, Height, Time};
use prost::Message;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::format;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;
use validation_context::ValidationParams;

use rlp_derive::{RlpDecodable, RlpEncodable};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateClientCommitment {
    pub prev_state_id: Option<StateID>,
    pub new_state_id: StateID,
    pub new_state: Option<Any>,
    pub prev_height: Option<Height>,
    pub new_height: Height,
    pub timestamp: Time,
    pub validation_params: ValidationParams,
}

impl Default for UpdateClientCommitment {
    fn default() -> Self {
        UpdateClientCommitment {
            prev_state_id: Default::default(),
            new_state_id: Default::default(),
            new_state: Default::default(),
            prev_height: Default::default(),
            new_height: Default::default(),
            timestamp: Time::unix_epoch(),
            validation_params: Default::default(),
        }
    }
}

// TODO can we avoid to define a substitute struct for RLP serialization?
#[derive(RlpEncodable, RlpDecodable, Default, Debug)]
pub struct RLPUpdateClientCommitment {
    prev_state_id: Vec<u8>,
    new_state_id: Vec<u8>,
    new_state: Vec<u8>,
    prev_height: Vec<u8>,
    new_height: Vec<u8>,
    timestamp: u128,
    validation_params: Vec<u8>,
}

impl UpdateClientCommitment {
    pub fn to_vec(&self) -> Vec<u8> {
        let c = RLPUpdateClientCommitment {
            prev_state_id: match &self.prev_state_id {
                Some(state_id) => state_id.to_vec(),
                None => vec![],
            },
            new_state_id: self.new_state_id.to_vec(),
            new_state: match self.new_state.as_ref() {
                Some(s) => s.encode_to_vec(),
                None => vec![],
            },
            prev_height: match self.prev_height {
                Some(h) => h.into(),
                None => vec![],
            },
            new_height: self.new_height.into(),
            timestamp: self.timestamp.as_unix_timestamp_nanos(),
            validation_params: self.validation_params.to_vec(),
        };
        rlp::encode(&c).to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let rc: RLPUpdateClientCommitment = rlp::decode(bz).map_err(Error::RLPDecoderError)?;
        Ok(Self {
            prev_state_id: match rc.prev_state_id {
                ref v if v.len() > 0 => Some(v.as_slice().try_into()?),
                _ => None,
            },
            new_state_id: rc.new_state_id.as_slice().try_into()?,
            new_state: match rc.new_state {
                v if v.len() > 0 => Some(Any::try_from(v).unwrap()),
                _ => None,
            },
            prev_height: match rc.prev_height.as_slice() {
                v if v.len() > 0 => Some(v.try_into()?),
                _ => None,
            },
            new_height: rc.new_height.as_slice().try_into()?,
            timestamp: Time::from_unix_timestamp_nanos(rc.timestamp)?,
            validation_params: ValidationParams::from_bytes(&rc.validation_params),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateCommitment {
    pub prefix: CommitmentPrefix,
    pub path: Path,
    pub value: Vec<u8>,
    pub height: Height,
    pub state_id: StateID,
}

impl StateCommitment {
    pub fn to_vec(self) -> Vec<u8> {
        let c = RLPStateCommitment {
            prefix: self.prefix.into_vec(),
            path: self.path.to_string(),
            value: self.value,
            height: self.height.into(),
            state_id: self.state_id.to_vec(),
        };
        rlp::encode(&c).to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let rc: RLPStateCommitment = rlp::decode(bz).map_err(Error::RLPDecoderError)?;
        Ok(Self {
            prefix: rc.prefix.try_into().map_err(Error::ICS23Error)?,
            path: Path::from_str(&rc.path).map_err(Error::ICS24PathError)?,
            value: rc.value,
            height: rc.height.as_slice().try_into()?,
            state_id: rc.state_id.as_slice().try_into()?,
        })
    }
}

#[derive(RlpEncodable, RlpDecodable, Default, Debug)]
pub struct RLPStateCommitment {
    pub prefix: Vec<u8>,
    pub path: String,
    pub value: Vec<u8>,
    pub height: Vec<u8>,
    pub state_id: Vec<u8>,
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::CommitmentError as Error;
use crate::{StateID, STATE_ID_SIZE};
use anyhow::{anyhow, Error as AnyhowError};
use core::str::FromStr;
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
            timestamp: Time::now(),
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
                Some(h) => height_to_bytes(h),
                None => vec![],
            },
            new_height: height_to_bytes(self.new_height),
            timestamp: self.timestamp.as_unix_timestamp_nanos(),
            validation_params: self.validation_params.to_vec(),
        };
        rlp::encode(&c).to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let rc: RLPUpdateClientCommitment = rlp::decode(bz).map_err(Error::RLPDecoderError)?;
        Ok(Self {
            prev_state_id: match rc.prev_state_id {
                ref v if v.len() > 0 => Some(bytes_to_state_id(v)?),
                _ => None,
            },
            new_state_id: bytes_to_state_id(&rc.new_state_id)?,
            new_state: match rc.new_state {
                v if v.len() > 0 => Some(Any::try_from(v).unwrap()),
                _ => None,
            },
            prev_height: match rc.prev_height {
                ref v if v.len() > 0 => Some(bytes_to_height(v)?),
                _ => None,
            },
            new_height: bytes_to_height(&rc.new_height)?,
            timestamp: Time::from_unix_timestamp_nanos(rc.timestamp).unwrap(),
            validation_params: ValidationParams::from_bytes(&rc.validation_params),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateCommitment {
    pub path: Path,
    pub value: Vec<u8>,
    pub height: Height,
    pub state_id: StateID,
}

impl StateCommitment {
    pub fn to_vec(&self) -> Vec<u8> {
        let c = RLPStateCommitment {
            path: self.path.to_string(),
            value: self.value.clone(),
            height: height_to_bytes(self.height),
            state_id: self.state_id.to_vec(),
        };
        rlp::encode(&c).to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let rc: RLPStateCommitment = rlp::decode(bz).map_err(Error::RLPDecoderError)?;
        Ok(Self {
            path: Path::from_str(&rc.path).map_err(Error::ICS24PathError)?,
            value: rc.value,
            height: bytes_to_height(&rc.height)?,
            state_id: bytes_to_state_id(&rc.state_id)?,
        })
    }
}

#[derive(RlpEncodable, RlpDecodable, Default, Debug)]
pub struct RLPStateCommitment {
    pub path: String,
    pub value: Vec<u8>,
    pub height: Vec<u8>,
    pub state_id: Vec<u8>,
}

fn height_to_bytes(h: Height) -> Vec<u8> {
    let mut bz: [u8; 16] = Default::default();
    bz[..8].copy_from_slice(&h.revision_number().to_be_bytes());
    bz[8..].copy_from_slice(&h.revision_height().to_be_bytes());
    bz.to_vec()
}

fn bytes_to_height(bz: &[u8]) -> Result<Height, AnyhowError> {
    if bz.len() != 16 {
        return Err(anyhow!("bytes length must be 16, but got {}", bz.len()));
    }
    let mut ar: [u8; 8] = Default::default();
    ar.copy_from_slice(&bz[..8]);
    let revision_number = u64::from_be_bytes(ar);
    ar.copy_from_slice(&bz[8..]);
    let revision_height = u64::from_be_bytes(ar);
    Ok(Height::new(revision_number, revision_height))
}

fn bytes_to_state_id(bz: &[u8]) -> Result<StateID, AnyhowError> {
    if bz.len() != STATE_ID_SIZE {
        return Err(anyhow!(
            "bytes length must be {}, but got {}",
            STATE_ID_SIZE,
            bz.len()
        ));
    }
    let mut ar: [u8; STATE_ID_SIZE] = Default::default();
    ar.copy_from_slice(bz);
    Ok(StateID::from_bytes_array(ar))
}

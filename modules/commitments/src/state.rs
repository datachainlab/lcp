use core::fmt::Display;

use crate::prelude::*;
use crate::Error;
use lcp_types::Any;
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::Digest;

pub const STATE_ID_SIZE: usize = 32;

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateID([u8; STATE_ID_SIZE]);

impl StateID {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Display for StateID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("0x{}", hex::encode(self.0)).as_str())
    }
}

impl From<[u8; STATE_ID_SIZE]> for StateID {
    fn from(value: [u8; STATE_ID_SIZE]) -> Self {
        Self(value)
    }
}

impl TryFrom<&[u8]> for StateID {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != STATE_ID_SIZE {
            return Err(Error::invalid_state_id_length(value.len()));
        }
        let mut bz: [u8; STATE_ID_SIZE] = Default::default();
        bz.copy_from_slice(value);
        Ok(Self(bz))
    }
}

pub fn gen_state_id_from_any(
    any_client_state: &Any,
    any_consensus_state: &Any,
) -> Result<StateID, Error> {
    let size = any_client_state.encoded_len() + any_consensus_state.encoded_len();
    let mut buf = vec![0; size];
    any_client_state.encode(&mut buf).unwrap();
    let offset = any_client_state.encoded_len();
    let mut slice = &mut buf[offset..];
    any_consensus_state.encode(&mut slice).unwrap();
    gen_state_id_from_bytes(&buf)
}

pub fn gen_state_id_from_bytes(bz: &[u8]) -> Result<StateID, Error> {
    let mut result: [u8; STATE_ID_SIZE] = Default::default();
    let h = sha2::Sha256::digest(bz).to_vec();
    result.copy_from_slice(&h);
    Ok(StateID(result))
}

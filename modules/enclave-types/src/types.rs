use crate::anyhow::{anyhow, Result};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use ibc::core::ics02_client::height::Height;
use ibc::core::ics24_host::identifier::ClientId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::format;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;

use rlp_derive::{RlpDecodable, RlpEncodable};

pub const STATE_ID_SIZE: usize = 32;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StateID([u8; STATE_ID_SIZE]);

impl StateID {
    pub fn from_bytes_array(bytes: [u8; STATE_ID_SIZE]) -> Self {
        StateID(bytes)
    }

    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ValidityProof {
    pub client_commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl ValidityProof {
    pub fn client_commitment(&self) -> ClientCommitment {
        ClientCommitment::from_rlp_bytes(&self.client_commitment_bytes).unwrap()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClientCommitment {
    pub client_id: ClientId,
    pub prev_state_id: Option<StateID>,
    pub new_state_id: StateID,
    pub prev_height: Option<Height>,
    pub new_height: Height,
    pub timestamp: u64,
}

#[derive(RlpEncodable, RlpDecodable, Default, Debug)]
pub struct RLPClientCommitment {
    client_id: String,
    prev_state_id: Vec<u8>,
    new_state_id: Vec<u8>,
    prev_height: Vec<u8>,
    new_height: Vec<u8>,
    timestamp: u64,
}

impl ClientCommitment {
    pub fn as_rlp_bytes(&self) -> Vec<u8> {
        let c = RLPClientCommitment {
            client_id: self.client_id.to_string(),
            prev_state_id: match &self.prev_state_id {
                Some(v) => v.0.to_vec(),
                None => vec![],
            },
            new_state_id: self.new_state_id.0.to_vec(),
            prev_height: match self.prev_height {
                Some(h) => height_to_bytes(h),
                None => vec![],
            },
            new_height: height_to_bytes(self.new_height),
            timestamp: self.timestamp,
        };
        let b = rlp::encode(&c);
        b.to_vec()
    }

    pub fn from_rlp_bytes(bz: &[u8]) -> Result<Self> {
        let rc: RLPClientCommitment = rlp::decode(bz).unwrap();
        Ok(Self {
            client_id: string_to_client_id(rc.client_id)?,
            prev_state_id: match rc.prev_state_id {
                ref v if v.len() > 0 => Some(bytes_to_state_id(v)?),
                _ => None,
            },
            new_state_id: bytes_to_state_id(&rc.new_state_id)?,
            prev_height: match rc.prev_height {
                ref v if v.len() > 0 => Some(bytes_to_height(v)?),
                _ => None,
            },
            new_height: bytes_to_height(&rc.new_height)?,
            timestamp: rc.timestamp,
        })
    }
}

fn string_to_client_id(client_id: String) -> Result<ClientId> {
    Ok(serde_json::from_value::<ClientId>(Value::String(client_id)).unwrap())
}

fn height_to_bytes(h: Height) -> Vec<u8> {
    let mut bz: [u8; 16] = Default::default();
    bz[..8].copy_from_slice(&h.revision_number.to_be_bytes());
    bz[8..].copy_from_slice(&h.revision_height.to_be_bytes());
    bz.to_vec()
}

fn bytes_to_height(bz: &[u8]) -> Result<Height> {
    if bz.len() != 16 {
        return Err(anyhow!("bytes length must be 16, but got {}", bz.len()));
    }
    let mut h = Height::default();
    let mut ar: [u8; 8] = Default::default();
    ar.copy_from_slice(&bz[..8]);
    h.revision_number = u64::from_be_bytes(ar);
    ar.copy_from_slice(&bz[8..]);
    h.revision_height = u64::from_be_bytes(ar);
    Ok(h)
}

fn bytes_to_state_id(bz: &[u8]) -> Result<StateID> {
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

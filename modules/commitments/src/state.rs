#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::types::{StateID, STATE_ID_SIZE};
use anyhow::Result;
use ibc::core::ics02_client::{client_consensus::AnyConsensusState, client_state::AnyClientState};
use prost::Message;
use prost_types::Any;
use sha2::{Digest, Sha256};
use std::vec;

// TODO define owned error types

pub fn gen_state_id(
    any_client_state: AnyClientState,
    any_consensus_state: AnyConsensusState,
) -> Result<StateID> {
    let any_client_state = Any::from(any_client_state);
    let any_consensus_state = Any::from(any_consensus_state);
    gen_state_id_from_any(&any_client_state, &any_consensus_state)
}

pub fn gen_state_id_from_any(any_client_state: &Any, any_consensus_state: &Any) -> Result<StateID> {
    let mut result: [u8; STATE_ID_SIZE] = Default::default();
    let size = any_client_state.encoded_len() + any_consensus_state.encoded_len();
    let mut buf = vec![0; size];
    any_client_state.encode(&mut buf).unwrap();
    let offset = any_client_state.encoded_len();
    let mut slice = &mut buf[offset..];
    any_consensus_state.encode(&mut slice).unwrap();

    let mut hasher = Sha256::new();
    hasher.input(&buf);
    let h = hasher.result();
    result.copy_from_slice(&h);
    Ok(StateID::from_bytes_array(result))
}

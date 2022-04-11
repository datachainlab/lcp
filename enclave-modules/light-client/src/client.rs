use crate::errors::Result;
use enclave_types::{StateID, STATE_ID_SIZE};
use ibc::{
    core::{
        ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState,
            context::ClientReader, height::Height,
        },
        ics03_connection::context::ConnectionReader,
        ics24_host::identifier::ClientId,
    },
    timestamp::Timestamp,
};
use prost::Message;
use prost_types::Any;
use sha2::{Digest, Sha256};
use std::string::String;
use std::vec;
use std::vec::Vec;

pub trait LightClient {
    fn create_client(
        &self,
        ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult>;
    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult>;
    fn verify_client(
        &self,
        ctx: &dyn ConnectionReader,
        client_id: ClientId,
        expected_client_state: Any,
        counterparty_prefix: Vec<u8>,
        counterparty_client_id: ClientId,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<VerifyClientResult>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateClientResult {
    pub client_id: ClientId,
    pub client_type: String,
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub height: Height,
    pub timestamp: Timestamp,
    pub processed_time: Timestamp,
    pub processed_height: Height,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateClientResult {
    pub client_id: ClientId,
    pub trusted_any_client_state: Any,
    pub trusted_any_consensus_state: Any,
    pub new_any_client_state: Any,
    pub new_any_consensus_state: Any,
    pub trusted_height: Height,
    pub height: Height,
    pub timestamp: Timestamp,
    pub processed_time: Timestamp,
    pub processed_height: Height,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifyClientResult {
    pub trusted_any_client_state: Any,
    pub trusted_any_consensus_state: Any,
}

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

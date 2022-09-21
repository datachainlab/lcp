#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{context::ClientReader, LightClientError};
use commitments::{StateCommitment, UpdateClientCommitment};
use ibc::core::ics24_host::identifier::ClientId;
use lcp_types::{Any, Height, Time};
use std::string::String;
use std::vec::Vec;

pub trait LightClient {
    fn client_type(&self) -> String;

    fn latest_height(
        &self,
        ctx: &dyn ClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError>;

    fn create_client(
        &self,
        ctx: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError>;

    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError>;

    fn verify_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError>;

    fn verify_non_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateClientResult {
    pub any_client_state: Any,
    pub any_consensus_state: Any,
    pub height: Height,
    pub timestamp: Time,
    pub commitment: UpdateClientCommitment,
    /// if true, sign the commitment with Enclave Key
    pub prove: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateClientResult {
    pub new_any_client_state: Any,
    pub new_any_consensus_state: Any,
    pub height: Height,
    pub timestamp: Time,
    pub commitment: UpdateClientCommitment,
    /// if true, sign the commitment with Enclave Key
    pub prove: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateVerificationResult {
    pub state_commitment: StateCommitment,
}

use crate::context::HostClientReader;
use crate::errors::Error;
use crate::prelude::*;
use commitments::{StateCommitment, UpdateClientCommitment};
use lcp_types::{Any, ClientId, Height};

pub trait LightClient {
    /// client_type returns a client type of the light client
    fn client_type(&self) -> String;

    /// latest_height returns the latest height that the light client tracks
    fn latest_height(
        &self,
        ctx: &dyn HostClientReader,
        client_id: &ClientId,
    ) -> Result<Height, Error>;

    /// create_client creates a new light client
    fn create_client(
        &self,
        ctx: &dyn HostClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, Error>;

    /// update_client updates the light client with a header
    fn update_client(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, Error>;

    /// verify_membership is a generic proof verification method which verifies a proof of the existence of a value at a given path at the specified height.
    fn verify_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, Error>;

    /// verify_non_membership is a generic proof verification method which verifies the absence of a given path at a specified height.
    fn verify_non_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateClientResult {
    /// height corresponding to the updated state
    pub height: Height,
    /// commitment represents a state transition of the client
    pub commitment: UpdateClientCommitment,
    /// if true, sign the commitment with Enclave Key
    pub prove: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateClientResult {
    /// updated client state
    pub new_any_client_state: Any,
    /// updated consensus state
    pub new_any_consensus_state: Any,
    /// height corresponding to the updated state
    pub height: Height,
    /// commitment represents a state transition of the client
    pub commitment: UpdateClientCommitment,
    /// if true, sign the commitment with Enclave Key
    pub prove: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateVerificationResult {
    /// state commitment represents a result of the state verification
    pub state_commitment: StateCommitment,
}

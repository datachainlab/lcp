use commitments::StateCommitmentProof;
use crypto::Keccak256;
use crypto::{verify_signature_address, Address};
use ibc::core::ics23_commitment::commitment::{CommitmentPrefix, CommitmentProofBytes};
use ibc::core::ics24_host::path::ClientConsensusStatePath;
use ibc_proto::ibc::core::commitment::v1::MerkleProof;
use ibc_proto::protobuf::Protobuf;
use lcp_types::{Any, ClientId, Height};
use light_client::HostClientReader;
use validation_context::{validation_predicate, ValidationContext};

use crate::client_state::ClientState;
use crate::consensus_state::ConsensusState;
use crate::errors::Error;
use crate::header::{Commitment, Header, RegisterEnclaveKeyHeader, UpdateClientHeader};
use crate::report::verify_report;

/// LCPClient is a PoC implementation of LCP Client
/// This is aimed to testing purposes only for now
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LCPClient {}

impl LCPClient {
    // check_header_and_update_state is called on UpdateClient
    pub fn check_header_and_update_state(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: Header,
    ) -> Result<(ClientState, ConsensusState), Error> {
        match header {
            Header::UpdateClient(header) => self.check_header_and_update_state_for_update_client(
                ctx,
                client_id,
                client_state,
                header,
            ),
            Header::RegisterEnclaveKey(header) => self
                .check_header_and_update_state_for_register_enclave_key(
                    ctx,
                    client_id,
                    client_state,
                    header,
                ),
        }
    }

    fn check_header_and_update_state_for_update_client(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: UpdateClientHeader,
    ) -> Result<(ClientState, ConsensusState), Error> {
        // TODO return an error instead of assertion

        if client_state.latest_height.is_zero() {
            // if the client state's latest height is zero, the commitment's new_state must be non-nil
            assert!(header.commitment.new_state.is_some());
        } else {
            // if the client state's latest height is non-zero, the commitment's prev_* must be non-nil
            assert!(header.prev_height().is_some() && header.prev_state_id().is_some());
            // check if the previous consensus state exists in the store
            let prev_consensus_state: ConsensusState = ctx
                .consensus_state(&client_id, &header.prev_height().unwrap())
                .unwrap()
                .try_into()?;
            assert!(prev_consensus_state.state_id == header.prev_state_id().unwrap());
        }

        // check if the specified signer exists in the client state
        assert!(client_state.contains(&header.signer()));

        // check if the `header.signer` matches the commitment prover
        let signer = verify_signature_address(&header.commitment_bytes, &header.signature).unwrap();
        assert!(header.signer() == signer);

        // check if proxy's validation context matches our's context
        let vctx = self.validation_context(ctx);
        assert!(validation_predicate(&vctx, header.validation_params()).unwrap());

        // create a new state
        let new_client_state = client_state.with_header(&header);
        let new_consensus_state = ConsensusState {
            state_id: header.state_id(),
            timestamp: header.timestamp(),
        };

        Ok((new_client_state, new_consensus_state))
    }

    fn check_header_and_update_state_for_register_enclave_key(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: RegisterEnclaveKeyHeader,
    ) -> Result<(ClientState, ConsensusState), Error> {
        // TODO return an error instead of assertion

        let vctx = self.validation_context(ctx);
        let eavr = header.0;
        let (key, attestation_time) = verify_report(&vctx, &client_state, &eavr).unwrap();

        let consensus_state = ConsensusState::try_from(
            ctx.consensus_state(&client_id, &client_state.latest_height)
                .unwrap(),
        )?;
        // TODO consider to improve sybil attack resistance for persmissionless environment
        let new_client_state = client_state.with_new_key((key, attestation_time));

        Ok((new_client_state, consensus_state))
    }

    pub fn verify_upgrade_and_update_state(
        &self,
        client_state: &ClientState,
        consensus_state: &ConsensusState,
        proof_upgrade_client: MerkleProof,
        proof_upgrade_consensus_state: MerkleProof,
    ) -> Result<(ClientState, ConsensusState), Error> {
        todo!()
    }

    fn validation_context(&self, ctx: &dyn HostClientReader) -> ValidationContext {
        ValidationContext::new(ctx.host_timestamp())
    }

    /// Verification functions as specified in:
    /// <https://github.com/cosmos/ibc/tree/master/spec/ics-002-client-semantics>
    ///
    /// Verify a `proof` that the consensus state of a given client (at height `consensus_height`)
    /// matches the input `consensus_state`. The parameter `counterparty_height` represent the
    /// height of the counterparty chain that this proof assumes (i.e., the height at which this
    /// proof was computed).
    #[allow(clippy::too_many_arguments)]
    pub fn verify_client_consensus_state(
        &self,
        ctx: &dyn HostClientReader,
        client_state: &ClientState,
        height: Height,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        client_id: &ClientId,
        consensus_height: Height,
        expected_consensus_state: &Any,
    ) -> Result<(), Error> {
        // TODO return an error instead of assertion

        // convert `proof` to StateCommitmentProof
        let commitment_proof: StateCommitmentProof = proof.clone().try_into().unwrap();
        let commitment = commitment_proof.commitment();

        // check if `.prefix` matches the counterparty connection's prefix
        assert!(&commitment.prefix == prefix);

        // check if `.path` matches expected the commitment path
        assert!(
            commitment.path
                == ClientConsensusStatePath {
                    client_id: client_id.clone().into(),
                    epoch: consensus_height.revision_number(),
                    height: consensus_height.revision_height(),
                }
                .into()
        );

        // check if `.height` matches proof height
        assert!(commitment.height == height);

        // check if `.value` matches expected state
        assert!(
            commitment.value == Some(expected_consensus_state.encode_vec().unwrap().keccak256())
        );

        // check if `.state_id` matches the corresponding stored consensus state's state_id
        let consensus_state =
            ConsensusState::try_from(ctx.consensus_state(client_id, &height).unwrap())?;
        assert!(consensus_state.state_id == commitment.state_id);

        // check if the `commitment_proof.signer` matches the commitment prover
        let signer = verify_signature_address(
            &commitment_proof.commitment_bytes,
            &commitment_proof.signature,
        )
        .unwrap();
        assert!(Address::from(&commitment_proof.signer as &[u8]) == signer);

        // check if the specified signer is not expired and exists in the client state
        let vctx = self.validation_context(ctx);
        assert!(client_state.is_active_key(vctx.current_timestamp, &signer));

        Ok(())
    }

    // initialise is called on CreateClient
    #[cfg(not(test))]
    pub fn initialise(
        &self,
        client_state: &ClientState,
        consensus_state: &ConsensusState,
    ) -> Result<(), Error> {
        // An initial client state's keys must be empty
        assert!(client_state.keys.len() == 0);
        // key_expiration must not be 0
        assert!(!client_state.key_expiration.is_zero());
        // An initial client state's latest height must be empty
        assert!(client_state.latest_height.is_zero());
        // mr_enclave length must be 32
        assert!(client_state.mr_enclave.len() == 32);
        // An initial consensus state must be empty
        assert!(consensus_state.is_empty());

        Ok(())
    }

    // WARNING: FOR ONLY TESTING PURPOSE: initialise is called on CreateClient
    #[cfg(test)]
    pub fn initialise(
        &self,
        client_state: &ClientState,
        consensus_state: &ConsensusState,
    ) -> Result<(), Error> {
        // An initial client state's keys must not be empty
        assert!(client_state.keys.len() != 0);
        // key_expiration must not be 0
        assert!(!client_state.key_expiration.is_zero());
        // An initial client state's latest height must be empty
        assert!(client_state.latest_height.is_zero());
        // mr_enclave length must be 0
        assert!(client_state.mr_enclave.len() == 0);
        // An initial consensus state must be empty
        assert!(consensus_state.is_empty());

        Ok(())
    }
}

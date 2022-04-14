use context::LightClientReader as ClientReader;
use ibc::core::ics02_client::client_consensus::AnyConsensusState;
use ibc::core::ics02_client::client_state::AnyClientState;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error as Ics02Error;
use ibc::core::ics02_client::header::Header as Ics02Header;
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics04_channel::context::ChannelReader;
use ibc::core::ics04_channel::packet::Sequence;
use ibc::core::ics23_commitment::commitment::{
    CommitmentPrefix, CommitmentProofBytes, CommitmentRoot,
};
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::Height;
use ibc_proto::ibc::core::commitment::v1::MerkleProof;
use validation_context::tendermint::TendermintValidationPredicate;
use validation_context::{ValidationContext, ValidationPredicate};

use crate::client_state::ClientState;
use crate::consensus_state::ConsensusState;
use crate::crypto::verify_signature;
use crate::header::{Header, RegisterEnclaveKeyHeader, UpdateClientHeader};
use crate::report::{read_enclave_key_from_report, verify_report};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LCPClient {}

impl LCPClient {
    pub fn check_header_and_update_state(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: Header,
    ) -> Result<(ClientState, ConsensusState), Ics02Error> {
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
        ctx: &dyn ClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: UpdateClientHeader,
    ) -> Result<(ClientState, ConsensusState), Ics02Error> {
        // TODO return an error instead of assertion

        // header validation
        assert!(header.prev_height().is_some() && header.prev_state_id().is_some());

        // check if the proxy's trusted consensus state exists in the store
        let prev_consensus_state: ConsensusState = ctx
            .consensus_state(&client_id, header.prev_height().unwrap())?
            .into();
        assert!(prev_consensus_state.state_id == header.prev_state_id().unwrap());

        // check if the specified signer exists in the client state
        assert!(client_state.contains(&header.signer()));

        // check if the `header.signer` matches the commitment prover
        let signer = verify_signature(&header.0.commitment_bytes, &header.0.signature).unwrap();
        assert!(header.signer() == signer);

        // check if proxy's validation context matches our's context
        let vctx = ValidationContext {
            current_timestamp: ctx
                .host_timestamp()
                .into_datetime()
                .unwrap()
                .unix_timestamp_nanos()
                .try_into()
                .unwrap(),
        };
        assert!(
            TendermintValidationPredicate::predicate(&vctx, header.validation_params()).unwrap()
        );

        // create a new state
        let new_client_state = client_state.with_header(&header);
        let new_consensus_state = ConsensusState {
            state_id: header.state_id(),
            timestamp: header.timestamp_as_u128(),
        };

        Ok((new_client_state, new_consensus_state))
    }

    fn check_header_and_update_state_for_register_enclave_key(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        client_state: ClientState,
        header: RegisterEnclaveKeyHeader,
    ) -> Result<(ClientState, ConsensusState), Ics02Error> {
        // TODO return an error instead of assertion

        assert!(verify_report(&client_state.mr_enclave, &header.0));
        let key = read_enclave_key_from_report(&header.0.body).unwrap();

        let any_consensus_state = ctx
            .consensus_state(&client_id, client_state.latest_height)
            .unwrap();
        let consensus_state = ConsensusState::from(any_consensus_state);
        let new_client_state = client_state.with_new_key(key);

        Ok((new_client_state, consensus_state))
    }

    pub fn verify_upgrade_and_update_state(
        &self,
        client_state: &ClientState,
        consensus_state: &ConsensusState,
        proof_upgrade_client: MerkleProof,
        proof_upgrade_consensus_state: MerkleProof,
    ) -> Result<(ClientState, ConsensusState), Ics02Error> {
        todo!()
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
        client_state: &ClientState,
        height: Height,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        client_id: &ClientId,
        consensus_height: Height,
        expected_consensus_state: &AnyConsensusState,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that a connection state matches that of the input `connection_end`.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_connection_state(
        &self,
        client_state: &ClientState,
        height: Height,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        connection_id: &ConnectionId,
        expected_connection_end: &ConnectionEnd,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that a channel state matches that of the input `channel_end`.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_channel_state(
        &self,
        client_state: &ClientState,
        height: Height,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        port_id: &PortId,
        channel_id: &ChannelId,
        expected_channel_end: &ChannelEnd,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify the client state for this chain that it is stored on the counterparty chain.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_client_full_state(
        &self,
        client_state: &ClientState,
        height: Height,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        client_id: &ClientId,
        expected_client_state: &AnyClientState,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that a packet has been commited.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_packet_data(
        &self,
        ctx: &dyn ChannelReader,
        client_state: &ClientState,
        height: Height,
        connection_end: &ConnectionEnd,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: Sequence,
        commitment: String,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that a packet has been commited.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_packet_acknowledgement(
        &self,
        ctx: &dyn ChannelReader,
        client_state: &ClientState,
        height: Height,
        connection_end: &ConnectionEnd,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: Sequence,
        ack: Vec<u8>,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that of the next_seq_received.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_next_sequence_recv(
        &self,
        ctx: &dyn ChannelReader,
        client_state: &ClientState,
        height: Height,
        connection_end: &ConnectionEnd,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: Sequence,
    ) -> Result<(), Ics02Error> {
        todo!()
    }

    /// Verify a `proof` that a packet has not been received.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_packet_receipt_absence(
        &self,
        ctx: &dyn ChannelReader,
        client_state: &ClientState,
        height: Height,
        connection_end: &ConnectionEnd,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: Sequence,
    ) -> Result<(), Ics02Error> {
        todo!()
    }
}

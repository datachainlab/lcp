use crate::prelude::*;
use crate::types::Height;
use crate::{ErrorDetail as LightClientErrorDetail, HostClientReader};
use core::marker::PhantomData;
use ibc::core::ics02_client::client_state::ClientState as Ics02ClientState;
use ibc::core::ics02_client::error::ClientError;
use ibc::core::ContextError;
use ibc::core::{
    context::Router, ics02_client::consensus_state::ConsensusState as Ics02ConsensusState,
    ValidationContext,
};
use lcp_proto::google::protobuf::Any as IBCAny;

/// IBCContext is a context that implements ValidationContext from ibc-rs over elc's context
/// NOTE: Since elc provides only 02-client equivalent functions, it implements only a very limited functions of ValidationContext trait.
pub struct IBCContext<'a, ClientState: Ics02ClientState, ConsensusState: Ics02ConsensusState> {
    parent: &'a dyn HostClientReader,
    marker: PhantomData<(ClientState, ConsensusState)>,
}

impl<'a, ClientState: Ics02ClientState, ConsensusState: Ics02ConsensusState>
    IBCContext<'a, ClientState, ConsensusState>
{
    pub fn new(parent: &'a dyn HostClientReader) -> Self {
        Self {
            parent,
            marker: Default::default(),
        }
    }
}

#[allow(unused_variables)]
impl<'a, ClientState: Ics02ClientState, ConsensusState: Ics02ConsensusState> ValidationContext
    for IBCContext<'a, ClientState, ConsensusState>
{
    fn client_state(
        &self,
        client_id: &ibc::core::ics24_host::identifier::ClientId,
    ) -> Result<
        alloc::boxed::Box<dyn ibc::core::ics02_client::client_state::ClientState>,
        ibc::core::ContextError,
    > {
        let any_client_state: IBCAny = self
            .parent
            .client_state(&client_id.clone().into())
            .map_err(|e| {
                ContextError::ClientError(ClientError::ClientSpecific {
                    description: e.to_string(),
                })
            })?
            .into();
        Ok(ClientState::try_from(any_client_state)?.into_box())
    }

    fn decode_client_state(
        &self,
        client_state: lcp_proto::google::protobuf::Any,
    ) -> Result<
        alloc::boxed::Box<dyn ibc::core::ics02_client::client_state::ClientState>,
        ibc::core::ContextError,
    > {
        let any = IBCAny {
            type_url: client_state.type_url,
            value: client_state.value,
        };
        Ok(ClientState::try_from(any)?.into_box())
    }

    fn consensus_state(
        &self,
        client_cons_state_path: &ibc::core::ics24_host::path::ClientConsensusStatePath,
    ) -> Result<
        alloc::boxed::Box<dyn ibc::core::ics02_client::consensus_state::ConsensusState>,
        ibc::core::ContextError,
    > {
        let height = Height::new(client_cons_state_path.epoch, client_cons_state_path.height);
        match self
            .parent
            .consensus_state(&client_cons_state_path.client_id.clone().into(), &height)
        {
            Ok(any_consensus_state) => {
                let any_consensus_state = IBCAny::from(any_consensus_state);
                let consensus_state = ConsensusState::try_from(any_consensus_state).unwrap();
                Ok(consensus_state.into_box())
            }
            Err(e) => match e.detail() {
                LightClientErrorDetail::ConsensusStateNotFound(d) => Err(
                    ContextError::ClientError(ClientError::ConsensusStateNotFound {
                        client_id: d.client_id.clone().into(),
                        height: d.height.try_into().unwrap(),
                    }),
                ),
                _ => Err(ContextError::ClientError(ClientError::ClientSpecific {
                    description: e.to_string(),
                })),
            },
        }
    }

    fn next_consensus_state(
        &self,
        client_id: &ibc::core::ics24_host::identifier::ClientId,
        height: &ibc::Height,
    ) -> Result<
        Option<alloc::boxed::Box<dyn ibc::core::ics02_client::consensus_state::ConsensusState>>,
        ibc::core::ContextError,
    > {
        Ok(None)
    }

    fn prev_consensus_state(
        &self,
        client_id: &ibc::core::ics24_host::identifier::ClientId,
        height: &ibc::Height,
    ) -> Result<
        Option<alloc::boxed::Box<dyn ibc::core::ics02_client::consensus_state::ConsensusState>>,
        ibc::core::ContextError,
    > {
        Ok(None)
    }

    fn host_height(&self) -> Result<ibc::Height, ibc::core::ContextError> {
        unimplemented!()
    }

    fn host_timestamp(&self) -> Result<ibc::timestamp::Timestamp, ibc::core::ContextError> {
        Ok(self.parent.host_timestamp().into())
    }

    fn host_consensus_state(
        &self,
        height: &ibc::Height,
    ) -> Result<
        alloc::boxed::Box<dyn ibc::core::ics02_client::consensus_state::ConsensusState>,
        ibc::core::ContextError,
    > {
        unimplemented!()
    }

    fn client_counter(&self) -> Result<u64, ibc::core::ContextError> {
        self.parent.client_counter().map_err(|e| {
            ContextError::ClientError(ClientError::ClientSpecific {
                description: e.to_string(),
            })
        })
    }

    fn connection_end(
        &self,
        conn_id: &ibc::core::ics24_host::identifier::ConnectionId,
    ) -> Result<ibc::core::ics03_connection::connection::ConnectionEnd, ibc::core::ContextError>
    {
        unimplemented!()
    }

    fn validate_self_client(
        &self,
        client_state_of_host_on_counterparty: lcp_proto::google::protobuf::Any,
    ) -> Result<(), ibc::core::ics03_connection::error::ConnectionError> {
        unimplemented!()
    }

    fn commitment_prefix(&self) -> ibc::core::ics23_commitment::commitment::CommitmentPrefix {
        unimplemented!()
    }

    fn connection_counter(&self) -> Result<u64, ibc::core::ContextError> {
        unimplemented!()
    }

    fn channel_end(
        &self,
        channel_end_path: &ibc::core::ics24_host::path::ChannelEndPath,
    ) -> Result<ibc::core::ics04_channel::channel::ChannelEnd, ibc::core::ContextError> {
        unimplemented!()
    }

    fn connection_channels(
        &self,
        cid: &ibc::core::ics24_host::identifier::ConnectionId,
    ) -> Result<
        alloc::vec::Vec<(
            ibc::core::ics24_host::identifier::PortId,
            ibc::core::ics24_host::identifier::ChannelId,
        )>,
        ibc::core::ContextError,
    > {
        unimplemented!()
    }

    fn get_next_sequence_send(
        &self,
        seq_send_path: &ibc::core::ics24_host::path::SeqSendPath,
    ) -> Result<ibc::core::ics04_channel::packet::Sequence, ibc::core::ContextError> {
        unimplemented!()
    }

    fn get_next_sequence_recv(
        &self,
        seq_recv_path: &ibc::core::ics24_host::path::SeqRecvPath,
    ) -> Result<ibc::core::ics04_channel::packet::Sequence, ibc::core::ContextError> {
        unimplemented!()
    }

    fn get_next_sequence_ack(
        &self,
        seq_ack_path: &ibc::core::ics24_host::path::SeqAckPath,
    ) -> Result<ibc::core::ics04_channel::packet::Sequence, ibc::core::ContextError> {
        unimplemented!()
    }

    fn get_packet_commitment(
        &self,
        commitment_path: &ibc::core::ics24_host::path::CommitmentPath,
    ) -> Result<ibc::core::ics04_channel::commitment::PacketCommitment, ibc::core::ContextError>
    {
        unimplemented!()
    }

    fn get_packet_receipt(
        &self,
        receipt_path: &ibc::core::ics24_host::path::ReceiptPath,
    ) -> Result<ibc::core::ics04_channel::packet::Receipt, ibc::core::ContextError> {
        unimplemented!()
    }

    fn get_packet_acknowledgement(
        &self,
        ack_path: &ibc::core::ics24_host::path::AckPath,
    ) -> Result<
        ibc::core::ics04_channel::commitment::AcknowledgementCommitment,
        ibc::core::ContextError,
    > {
        unimplemented!()
    }

    fn hash(&self, value: &[u8]) -> alloc::vec::Vec<u8> {
        unimplemented!()
    }

    fn client_update_time(
        &self,
        client_id: &ibc::core::ics24_host::identifier::ClientId,
        height: &ibc::Height,
    ) -> Result<ibc::timestamp::Timestamp, ibc::core::ContextError> {
        unimplemented!()
    }

    fn client_update_height(
        &self,
        client_id: &ibc::core::ics24_host::identifier::ClientId,
        height: &ibc::Height,
    ) -> Result<ibc::Height, ibc::core::ContextError> {
        unimplemented!()
    }

    fn channel_counter(&self) -> Result<u64, ibc::core::ContextError> {
        unimplemented!()
    }

    fn max_expected_time_per_block(&self) -> core::time::Duration {
        unimplemented!()
    }
}

#[allow(unused_variables)]
impl<'a, ClientState: Ics02ClientState, ConsensusState: Ics02ConsensusState> Router
    for IBCContext<'a, ClientState, ConsensusState>
{
    fn get_route(
        &self,
        module_id: &ibc::core::ics26_routing::context::ModuleId,
    ) -> Option<&dyn ibc::core::ics26_routing::context::Module> {
        unimplemented!()
    }

    fn get_route_mut(
        &mut self,
        module_id: &ibc::core::ics26_routing::context::ModuleId,
    ) -> Option<&mut dyn ibc::core::ics26_routing::context::Module> {
        unimplemented!()
    }

    fn has_route(&self, module_id: &ibc::core::ics26_routing::context::ModuleId) -> bool {
        unimplemented!()
    }

    fn lookup_module_by_port(
        &self,
        port_id: &ibc::core::ics24_host::identifier::PortId,
    ) -> Option<ibc::core::ics26_routing::context::ModuleId> {
        unimplemented!()
    }
}

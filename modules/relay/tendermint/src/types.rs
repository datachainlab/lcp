use ibc::{
    clients::ics07_tendermint::{
        client_state::ClientState as TendermintClientState,
        consensus_state::ConsensusState as TendermintConsensusState, header::Header,
    },
    core::{
        ics03_connection::connection::ConnectionEnd,
        ics04_channel::{channel::ChannelEnd, packet::Sequence},
        ics24_host::identifier::{ChainId, ChannelId, ConnectionId, PortId},
    },
    Height,
};
use ibc_proto::{google::protobuf::Any as IBCAny, protobuf::Protobuf};
use ibc_proto_relayer::{
    google::protobuf::Any as IBCRelayerAny, protobuf::Protobuf as RelayerProtobuf,
};
use ibc_relayer_types::core::ics04_channel::packet::Sequence as RSequence;
use ibc_relayer_types::core::ics24_host::identifier::{
    ChannelId as RChannelId, ConnectionId as RConnectionId, PortId as RPortId,
};
use ibc_relayer_types::{
    clients::ics07_tendermint::{
        client_state::ClientState as RTendermintClientState,
        consensus_state::ConsensusState as RTendermintConsensusState, header::Header as RHeader,
    },
    core::{
        ics03_connection::connection::ConnectionEnd as RConnectionEnd,
        ics04_channel::channel::ChannelEnd as RChannelEnd,
    },
};
use ibc_relayer_types::{core::ics24_host::identifier::ChainId as RChainId, Height as RHeight};
use lcp_types::Any;
use std::str::FromStr;

pub fn relayer_header_to_any(value: RHeader) -> Any {
    let any = IBCRelayerAny::from(value);
    Any::new(any.type_url, any.value)
}

/// relayer-types to ibc

pub fn to_ibc_connection(value: RConnectionEnd) -> ConnectionEnd {
    ConnectionEnd::decode_vec(&value.encode_vec().unwrap()).unwrap()
}

pub fn to_ibc_connection_id(value: RConnectionId) -> ConnectionId {
    ConnectionId::from_str(value.as_str()).unwrap()
}

pub fn to_ibc_channel(value: RChannelEnd) -> ChannelEnd {
    ChannelEnd::decode_vec(&value.encode_vec().unwrap()).unwrap()
}

pub fn to_ibc_channel_id(value: RChannelId) -> ChannelId {
    ChannelId::from_str(value.as_str()).unwrap()
}

pub fn to_ibc_port_id(value: RPortId) -> PortId {
    PortId::from_str(value.as_str()).unwrap()
}

pub fn to_ibc_height(value: RHeight) -> Height {
    Height::new(value.revision_number(), value.revision_height()).unwrap()
}

pub fn to_ibc_client_state(value: RTendermintClientState) -> TendermintClientState {
    let any = IBCRelayerAny::from(value);
    TendermintClientState::try_from(IBCAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

pub fn to_ibc_consensus_state(value: RTendermintConsensusState) -> TendermintConsensusState {
    let any = IBCRelayerAny::from(value);
    TendermintConsensusState::try_from(IBCAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

pub fn to_ibc_header(value: RHeader) -> Header {
    let any = IBCRelayerAny::from(value);
    Header::try_from(IBCAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

/// ibc to relayer-types

pub fn to_relayer_chain_id(value: ChainId) -> RChainId {
    RChainId::from_str(value.as_str()).unwrap()
}

pub fn to_relayer_height(value: Height) -> RHeight {
    RHeight::new(value.revision_number(), value.revision_height()).unwrap()
}

pub fn to_relayer_connection_id(value: ConnectionId) -> RConnectionId {
    RConnectionId::from_str(value.as_str()).unwrap()
}

pub fn to_relayer_channel_id(value: ChannelId) -> RChannelId {
    RChannelId::from_str(value.as_str()).unwrap()
}

pub fn to_relayer_port_id(value: PortId) -> RPortId {
    RPortId::from_str(value.as_str()).unwrap()
}

pub fn to_relayer_sequence(value: Sequence) -> RSequence {
    RSequence::from_str(value.to_string().as_str()).unwrap()
}

pub fn to_relayer_client_state(value: TendermintClientState) -> RTendermintClientState {
    let any = IBCAny::from(value);
    RTendermintClientState::try_from(IBCRelayerAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

pub fn to_relayer_consensus_state(value: TendermintConsensusState) -> RTendermintConsensusState {
    let any = IBCAny::from(value);
    RTendermintConsensusState::try_from(IBCRelayerAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

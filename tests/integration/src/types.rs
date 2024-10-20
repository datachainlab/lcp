use ibc::{
    clients::ics07_tendermint::{
        client_state::ClientState as TendermintClientState,
        consensus_state::ConsensusState as TendermintConsensusState,
    },
    core::{
        ics04_channel::channel::ChannelEnd,
        ics24_host::identifier::{ChainId, ChannelId, PortId},
    },
    Height,
};
use ibc_proto_relayer24::google::protobuf::Any as ProtoAny24;
use ibc_proto_relayer24::protobuf::Protobuf as Protobuf24;
use ibc_proto_relayer26::{
    google::protobuf::Any as IBCRelayerAny, protobuf::Protobuf as RelayerProtobuf,
};
use ibc_relayer_types::core::ics24_host::identifier::{ChannelId as RChannelId, PortId as RPortId};
use ibc_relayer_types::{
    clients::ics07_tendermint::{
        client_state::ClientState as RTendermintClientState,
        consensus_state::ConsensusState as RTendermintConsensusState, header::Header as RHeader,
    },
    core::ics04_channel::channel::ChannelEnd as RChannelEnd,
};
use ibc_relayer_types::{core::ics24_host::identifier::ChainId as RChainId, Height as RHeight};
// use lcp_proto::{google::protobuf::Any as ProtoAny};
use lcp_types::Any;
use std::str::FromStr;

/// WARNING: The following converters are very inefficient, so they should not be used except for testing purpose.
/// ibc-relayer(hermes) has owned ibc crate, not cosmos/ibc-rs. Therefore, the following converters are required for now.

/// relayer-types to lcp types

pub(crate) fn relayer_header_to_any(value: RHeader) -> Any {
    let any = ProtoAny24::from(value);
    Any::new(any.type_url, any.value)
}

/// relayer-types to ibc

pub(crate) fn to_ibc_channel(value: RChannelEnd) -> ChannelEnd {
    ChannelEnd::decode_vec(&value.encode_vec().unwrap()).unwrap()
}

pub(crate) fn to_ibc_height(value: RHeight) -> Height {
    Height::new(value.revision_number(), value.revision_height()).unwrap()
}

pub(crate) fn to_ibc_client_state(value: RTendermintClientState) -> TendermintClientState {
    let any = ProtoAny24::from(value);
    TendermintClientState::try_from(IBCRelayerAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

pub(crate) fn to_ibc_consensus_state(value: RTendermintConsensusState) -> TendermintConsensusState {
    let any = ProtoAny24::from(value);
    TendermintConsensusState::try_from(IBCRelayerAny {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

/// ibc to relayer-types

pub(crate) fn to_relayer_chain_id(value: ChainId) -> RChainId {
    RChainId::from_str(value.as_str()).unwrap()
}

pub(crate) fn to_relayer_height(value: Height) -> RHeight {
    RHeight::new(value.revision_number(), value.revision_height()).unwrap()
}

pub(crate) fn to_relayer_channel_id(value: ChannelId) -> RChannelId {
    RChannelId::from_str(value.as_str()).unwrap()
}

pub(crate) fn to_relayer_port_id(value: PortId) -> RPortId {
    RPortId::from_str(value.as_str()).unwrap()
}

pub(crate) fn to_relayer_client_state(value: TendermintClientState) -> RTendermintClientState {
    let any = IBCRelayerAny::from(value);
    RTendermintClientState::try_from(ProtoAny24 {
        type_url: any.type_url,
        value: any.value,
    })
    .unwrap()
}

pub(crate) fn any_to_any(a0: IBCRelayerAny) -> Any {
    Any::new(a0.type_url, a0.value)
}

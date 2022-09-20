use crate::lcp::service::elc::v1::{
    MsgVerifyMembership, MsgVerifyMembershipResponse, MsgVerifyNonMembership,
    MsgVerifyNonMembershipResponse,
};
use crate::lcp::service::ibc::v1::{
    MsgVerificationResponse, MsgVerifyChannel, MsgVerifyClient, MsgVerifyClientConsensus,
    MsgVerifyConnection, MsgVerifyNextSequenceRecv, MsgVerifyPacket,
    MsgVerifyPacketAcknowledgement, MsgVerifyPacketReceiptAbsense,
};
use alloc::string::ToString;
use core::str::FromStr;
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::core::ics24_host::path::{
    AcksPath, ChannelEndsPath, ClientConsensusStatePath, ClientStatePath, CommitmentsPath,
    ConnectionsPath, SeqRecvsPath,
};
use ibc::core::ics24_host::Path;
use prost::Message;
use tonic::Status;

impl TryFrom<MsgVerifyClient> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyClient) -> Result<Self, Status> {
        let counterparty_client_id =
            ClientId::from_str(&msg.counterparty_client_id).map_err(|e| {
                Status::invalid_argument(format!("invalid counterparty_client_id: err={}", e))
            })?;
        let path = Path::ClientState(ClientStatePath(counterparty_client_id)).to_string();
        let value = msg
            .target_any_client_state
            .ok_or(Status::invalid_argument(
                "target_any_client_state must be non-nil",
            ))?
            .encode_to_vec();
        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyClientConsensus> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyClientConsensus) -> Result<Self, Status> {
        let counterparty_client_id =
            ClientId::from_str(&msg.counterparty_client_id).map_err(|e| {
                Status::invalid_argument(format!("invalid counterparty_client_id: err={}", e))
            })?;
        let counterparty_consensus_height =
            msg.counterparty_consensus_height
                .ok_or(Status::invalid_argument(
                    "counterparty_consensus_height must be non-nil",
                ))?;
        let path = Path::ClientConsensusState(ClientConsensusStatePath {
            client_id: counterparty_client_id,
            epoch: counterparty_consensus_height.revision_number,
            height: counterparty_consensus_height.revision_height,
        })
        .to_string();
        let value = msg
            .target_any_client_consensus_state
            .ok_or(Status::invalid_argument(
                "target_any_client_state must be non-nil",
            ))?
            .encode_to_vec();
        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyConnection> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyConnection) -> Result<Self, Status> {
        let counterparty_connection_id = ConnectionId::from_str(&msg.counterparty_connection_id)
            .map_err(|e| {
                Status::invalid_argument(format!("invalid counterparty_connection_id: err={}", e))
            })?;
        let path = Path::Connections(ConnectionsPath(counterparty_connection_id)).to_string();

        let value = msg
            .expected_connection
            .ok_or(Status::invalid_argument(
                "expected_connection must be non-nil",
            ))?
            .encode_to_vec();

        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyChannel> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyChannel) -> Result<Self, Status> {
        let port_id = PortId::from_str(&msg.counterparty_port_id)
            .map_err(|e| Status::invalid_argument(format!("invalid port_id: err={}", e)))?;
        let channel_id = ChannelId::from_str(&msg.counterparty_channel_id)
            .map_err(|e| Status::invalid_argument(format!("invalid channel_id: err={}", e)))?;
        let path = Path::ChannelEnds(ChannelEndsPath(port_id, channel_id)).to_string();
        let value = msg
            .expected_channel
            .ok_or(Status::invalid_argument("expected_channel must be non-nil"))?
            .encode_to_vec();

        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyPacket> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyPacket) -> Result<Self, Status> {
        let port_id = PortId::from_str(&msg.port_id)
            .map_err(|e| Status::invalid_argument(format!("invalid port_id: err={}", e)))?;
        let channel_id = ChannelId::from_str(&msg.channel_id)
            .map_err(|e| Status::invalid_argument(format!("invalid channel_id: err={}", e)))?;
        let path = Path::Commitments(CommitmentsPath {
            port_id,
            channel_id,
            sequence: msg.sequence.into(),
        })
        .to_string();
        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value: msg.commitment,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyPacketAcknowledgement> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyPacketAcknowledgement) -> Result<Self, Status> {
        let port_id = PortId::from_str(&msg.port_id)
            .map_err(|e| Status::invalid_argument(format!("invalid port_id: err={}", e)))?;
        let channel_id = ChannelId::from_str(&msg.channel_id)
            .map_err(|e| Status::invalid_argument(format!("invalid channel_id: err={}", e)))?;

        let path = Path::Acks(AcksPath {
            port_id,
            channel_id,
            sequence: msg.sequence.into(),
        })
        .to_string();

        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value: msg.acknowledgement,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyPacketReceiptAbsense> for MsgVerifyNonMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyPacketReceiptAbsense) -> Result<Self, Status> {
        let port_id = PortId::from_str(&msg.port_id)
            .map_err(|e| Status::invalid_argument(format!("invalid port_id: err={}", e)))?;
        let channel_id = ChannelId::from_str(&msg.channel_id)
            .map_err(|e| Status::invalid_argument(format!("invalid channel_id: err={}", e)))?;
        let path = Path::Commitments(CommitmentsPath {
            port_id,
            channel_id,
            sequence: msg.sequence.into(),
        })
        .to_string();

        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl TryFrom<MsgVerifyNextSequenceRecv> for MsgVerifyMembership {
    type Error = Status;

    fn try_from(msg: MsgVerifyNextSequenceRecv) -> Result<Self, Status> {
        let port_id = PortId::from_str(&msg.port_id)
            .map_err(|e| Status::invalid_argument(format!("invalid port_id: err={}", e)))?;
        let channel_id = ChannelId::from_str(&msg.channel_id)
            .map_err(|e| Status::invalid_argument(format!("invalid channel_id: err={}", e)))?;

        let path = Path::SeqRecvs(SeqRecvsPath(port_id, channel_id)).to_string();
        let value = msg.next_sequence_recv.to_be_bytes().to_vec();
        Ok(Self {
            client_id: msg.client_id,
            prefix: msg.prefix,
            path,
            value,
            proof_height: msg.proof_height,
            proof: msg.proof,
        })
    }
}

impl From<MsgVerifyMembershipResponse> for MsgVerificationResponse {
    fn from(msg: MsgVerifyMembershipResponse) -> Self {
        Self {
            commitment: msg.commitment,
            signer: msg.signer,
            signature: msg.signature,
        }
    }
}

impl From<MsgVerifyNonMembershipResponse> for MsgVerificationResponse {
    fn from(msg: MsgVerifyNonMembershipResponse) -> Self {
        Self {
            commitment: msg.commitment,
            signer: msg.signer,
            signature: msg.signature,
        }
    }
}

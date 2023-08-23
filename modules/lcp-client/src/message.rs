use crate::errors::Error;
use crate::prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use ibc_proto::protobuf::Protobuf;
use lcp_proto::ibc::lightclients::lcp::v1::{
    RegisterEnclaveKeyMessage as RawRegisterEnclaveKeyMessage,
    UpdateClientMessage as RawUpdateClientMessage,
};
use lcp_types::{Any, Height, Time};
use light_client::commitments::{Commitment, CommitmentContext, StateID, UpdateClientCommitment};
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};

pub const LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.RegisterEnclaveKeyMessage";
pub const LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.UpdateClientMessage";

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ClientMessage {
    RegisterEnclaveKey(RegisterEnclaveKeyMessage),
    UpdateClient(UpdateClientMessage),
}

impl Protobuf<ProtoAny> for ClientMessage {}

impl TryFrom<ProtoAny> for ClientMessage {
    type Error = Error;

    fn try_from(raw: ProtoAny) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL => Ok(ClientMessage::RegisterEnclaveKey(
                RegisterEnclaveKeyMessage::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
            )),
            LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL => Ok(ClientMessage::UpdateClient(
                UpdateClientMessage::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
            )),
            _ => Err(Error::unexpected_header_type(raw.type_url)),
        }
    }
}

impl From<ClientMessage> for ProtoAny {
    fn from(value: ClientMessage) -> Self {
        match value {
            ClientMessage::RegisterEnclaveKey(h) => ProtoAny {
                type_url: LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL.to_string(),
                value: h.encode_vec().unwrap(),
            },
            ClientMessage::UpdateClient(h) => ProtoAny {
                type_url: LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL.to_string(),
                value: h.encode_vec().unwrap(),
            },
        }
    }
}

impl TryFrom<Any> for ClientMessage {
    type Error = Error;

    fn try_from(any: Any) -> Result<Self, Self::Error> {
        TryFrom::<ProtoAny>::try_from(any.into())
    }
}

impl From<ClientMessage> for Any {
    fn from(value: ClientMessage) -> Self {
        ProtoAny::from(value).try_into().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterEnclaveKeyMessage(pub EndorsedAttestationVerificationReport);

impl Protobuf<RawRegisterEnclaveKeyMessage> for RegisterEnclaveKeyMessage {}

impl TryFrom<RawRegisterEnclaveKeyMessage> for RegisterEnclaveKeyMessage {
    type Error = Error;
    fn try_from(value: RawRegisterEnclaveKeyMessage) -> Result<Self, Self::Error> {
        Ok(RegisterEnclaveKeyMessage(
            EndorsedAttestationVerificationReport {
                avr: value.report,
                signature: value.signature,
                signing_cert: value.signing_cert,
            },
        ))
    }
}

impl From<RegisterEnclaveKeyMessage> for RawRegisterEnclaveKeyMessage {
    fn from(value: RegisterEnclaveKeyMessage) -> Self {
        RawRegisterEnclaveKeyMessage {
            report: (&value.0.avr).try_into().unwrap(),
            signature: value.0.signature,
            signing_cert: value.0.signing_cert,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UpdateClientMessage {
    pub commitment_bytes: Vec<u8>,
    pub signer: Address,
    pub signature: Vec<u8>,
    pub commitment: UpdateClientCommitment,
}

impl Protobuf<RawUpdateClientMessage> for UpdateClientMessage {}

impl TryFrom<RawUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(value: RawUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(UpdateClientMessage {
            signer: Address::try_from(value.signer.as_slice())?,
            signature: value.signature,
            commitment: Commitment::from_commitment_bytes(&value.commitment)?.try_into()?,
            commitment_bytes: value.commitment,
        })
    }
}

impl From<UpdateClientMessage> for RawUpdateClientMessage {
    fn from(value: UpdateClientMessage) -> Self {
        RawUpdateClientMessage {
            commitment: Into::<Commitment>::into(value.commitment).to_commitment_bytes(),
            signer: value.signer.into(),
            signature: value.signature,
        }
    }
}

impl CommitmentReader for UpdateClientMessage {
    fn signer(&self) -> Address {
        self.signer
    }

    fn commitment(&self) -> &UpdateClientCommitment {
        &self.commitment
    }
}

pub trait CommitmentReader {
    fn signer(&self) -> Address;

    fn commitment(&self) -> &UpdateClientCommitment;

    fn height(&self) -> Height {
        self.commitment().new_height
    }

    fn prev_height(&self) -> Option<Height> {
        self.commitment().prev_height
    }

    fn prev_state_id(&self) -> Option<StateID> {
        self.commitment().prev_state_id
    }

    fn state_id(&self) -> StateID {
        self.commitment().new_state_id
    }

    fn timestamp(&self) -> Time {
        self.commitment().timestamp
    }

    fn context(&self) -> &CommitmentContext {
        &self.commitment().context
    }
}

impl ClientMessage {
    pub fn get_height(&self) -> Option<Height> {
        match self {
            ClientMessage::UpdateClient(h) => Some(h.height()),
            _ => None,
        }
    }

    pub fn get_timestamp(&self) -> Option<Time> {
        match self {
            ClientMessage::UpdateClient(h) => Some(h.timestamp()),
            _ => None,
        }
    }
}

use crate::errors::Error;
use crate::prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use light_client::commitments::{
    Message as ELCMessage, StateID, UpdateClientMessage as ELCUpdateClientMessage,
    ValidationContext,
};
use light_client::types::proto::ibc::lightclients::lcp::v1::{
    RegisterEnclaveKeyMessage as RawRegisterEnclaveKeyMessage,
    UpdateClientMessage as RawUpdateClientMessage,
};
use light_client::types::proto::protobuf::Protobuf;
use light_client::types::{Any, Height, Time};
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

impl Protobuf<Any> for ClientMessage {}

impl TryFrom<Any> for ClientMessage {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL => Ok(ClientMessage::RegisterEnclaveKey(
                RegisterEnclaveKeyMessage::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
            )),
            LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL => Ok(ClientMessage::UpdateClient(
                UpdateClientMessage::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
            )),
            type_url => Err(Error::unexpected_header_type(type_url.to_owned())),
        }
    }
}

impl From<ClientMessage> for Any {
    fn from(value: ClientMessage) -> Self {
        match value {
            ClientMessage::RegisterEnclaveKey(h) => Any::new(
                LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL.to_string(),
                h.encode_vec().unwrap(),
            ),
            ClientMessage::UpdateClient(h) => Any::new(
                LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL.to_string(),
                h.encode_vec().unwrap(),
            ),
        }
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
    pub signer: Address,
    pub signature: Vec<u8>,
    pub elc_message: ELCUpdateClientMessage,
}

impl Protobuf<RawUpdateClientMessage> for UpdateClientMessage {}

impl TryFrom<RawUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(value: RawUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(UpdateClientMessage {
            signer: Address::try_from(value.signer.as_slice())?,
            signature: value.signature,
            elc_message: ELCMessage::from_bytes(&value.elc_message)?.try_into()?,
        })
    }
}

impl From<UpdateClientMessage> for RawUpdateClientMessage {
    fn from(value: UpdateClientMessage) -> Self {
        RawUpdateClientMessage {
            elc_message: Into::<ELCMessage>::into(value.elc_message).to_bytes(),
            signer: value.signer.into(),
            signature: value.signature,
        }
    }
}

impl ELCMessageReader for UpdateClientMessage {
    fn signer(&self) -> Address {
        self.signer
    }

    fn elc_message(&self) -> &ELCUpdateClientMessage {
        &self.elc_message
    }
}

pub trait ELCMessageReader {
    fn signer(&self) -> Address;

    fn elc_message(&self) -> &ELCUpdateClientMessage;

    fn elc_message_bytes(&self) -> Vec<u8> {
        ELCMessage::from(self.elc_message().clone()).to_bytes()
    }

    fn height(&self) -> Height {
        self.elc_message().post_height
    }

    fn prev_height(&self) -> Option<Height> {
        self.elc_message().prev_height
    }

    fn prev_state_id(&self) -> Option<StateID> {
        self.elc_message().prev_state_id
    }

    fn state_id(&self) -> StateID {
        self.elc_message().post_state_id
    }

    fn timestamp(&self) -> Time {
        self.elc_message().timestamp
    }

    fn context(&self) -> &ValidationContext {
        &self.elc_message().context
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

use crate::errors::Error;
use crate::prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use light_client::commitments::ProxyMessage;
use light_client::types::proto::ibc::lightclients::lcp::v1::{
    RegisterEnclaveKeyMessage as RawRegisterEnclaveKeyMessage,
    UpdateClientMessage as RawUpdateClientMessage,
};
use light_client::types::{proto::protobuf::Protobuf, Any};
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
    pub proxy_message: ProxyMessage,
}

impl Protobuf<RawUpdateClientMessage> for UpdateClientMessage {}

impl TryFrom<RawUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(value: RawUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(UpdateClientMessage {
            signer: Address::try_from(value.signer.as_slice())?,
            signature: value.signature,
            proxy_message: ProxyMessage::from_bytes(&value.proxy_message)?,
        })
    }
}

impl From<UpdateClientMessage> for RawUpdateClientMessage {
    fn from(value: UpdateClientMessage) -> Self {
        RawUpdateClientMessage {
            proxy_message: Into::<ProxyMessage>::into(value.proxy_message).to_bytes(),
            signer: value.signer.into(),
            signature: value.signature,
        }
    }
}

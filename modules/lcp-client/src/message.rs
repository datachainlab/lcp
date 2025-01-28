use crate::errors::Error;
use crate::prelude::*;
use alloy_sol_types::{sol, SolValue};
use attestation_report::IASSignedReport;
use crypto::Address;
use dcap_rs::types::VerifiedOutput;
use light_client::commitments::{Error as CommitmentError, EthABIEncoder, ProxyMessage};
use light_client::types::proto::ibc::lightclients::lcp::v1::{
    RegisterEnclaveKeyMessage as RawRegisterEnclaveKeyMessage,
    UpdateClientMessage as RawUpdateClientMessage,
    UpdateOperatorsMessage as RawUpdateOperatorsMessage,
    ZkdcapRegisterEnclaveKeyMessage as RawZKDCAPRegisterEnclaveKeyMessage,
};
use light_client::types::{proto::protobuf::Protobuf, Any};
use serde::{Deserialize, Serialize};

pub const LCP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.RegisterEnclaveKeyMessage";
pub const LCP_ZKDCAP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.ZKDCAPRegisterEnclaveKeyMessage";
pub const LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.UpdateClientMessage";
pub const LCP_UPDATE_OPERATORS_MESSAGE_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.UpdateOperatorsMessage";

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ClientMessage {
    RegisterEnclaveKey(RegisterEnclaveKeyMessage),
    ZKDCAPRegisterEnclaveKey(ZKDCAPRegisterEnclaveKeyMessage),
    UpdateClient(UpdateClientMessage),
    UpdateOperators(UpdateOperatorsMessage),
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
            LCP_UPDATE_OPERATORS_MESSAGE_TYPE_URL => Ok(ClientMessage::UpdateOperators(
                UpdateOperatorsMessage::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
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
            ClientMessage::ZKDCAPRegisterEnclaveKey(h) => Any::new(
                LCP_ZKDCAP_REGISTER_ENCLAVE_KEY_MESSAGE_TYPE_URL.to_string(),
                h.encode_vec().unwrap(),
            ),
            ClientMessage::UpdateClient(h) => Any::new(
                LCP_UPDATE_CLIENT_MESSAGE_TYPE_URL.to_string(),
                h.encode_vec().unwrap(),
            ),
            ClientMessage::UpdateOperators(h) => Any::new(
                LCP_UPDATE_OPERATORS_MESSAGE_TYPE_URL.to_string(),
                h.encode_vec().unwrap(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterEnclaveKeyMessage {
    pub report: IASSignedReport,
    pub operator_signature: Option<Vec<u8>>,
}

impl Protobuf<RawRegisterEnclaveKeyMessage> for RegisterEnclaveKeyMessage {}

impl TryFrom<RawRegisterEnclaveKeyMessage> for RegisterEnclaveKeyMessage {
    type Error = Error;
    fn try_from(value: RawRegisterEnclaveKeyMessage) -> Result<Self, Self::Error> {
        Ok(RegisterEnclaveKeyMessage {
            report: IASSignedReport {
                avr: String::from_utf8(value.report)?,
                signature: value.signature,
                signing_cert: value.signing_cert,
            },
            operator_signature: (!value.operator_signature.is_empty())
                .then_some(value.operator_signature),
        })
    }
}

impl From<RegisterEnclaveKeyMessage> for RawRegisterEnclaveKeyMessage {
    fn from(value: RegisterEnclaveKeyMessage) -> Self {
        RawRegisterEnclaveKeyMessage {
            report: value.report.avr.into_bytes(),
            signature: value.report.signature,
            signing_cert: value.report.signing_cert,
            operator_signature: value.operator_signature.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZKDCAPRegisterEnclaveKeyMessage {
    pub commit: VerifiedOutput,
    pub proof: Vec<u8>,
    pub operator_signature: Option<Vec<u8>>,
}

impl Protobuf<RawZKDCAPRegisterEnclaveKeyMessage> for ZKDCAPRegisterEnclaveKeyMessage {}

impl TryFrom<RawZKDCAPRegisterEnclaveKeyMessage> for ZKDCAPRegisterEnclaveKeyMessage {
    type Error = Error;
    fn try_from(value: RawZKDCAPRegisterEnclaveKeyMessage) -> Result<Self, Self::Error> {
        Ok(ZKDCAPRegisterEnclaveKeyMessage {
            commit: VerifiedOutput::from_bytes(&value.commit)
                .map_err(Error::dcap_quote_verifier)?,
            proof: value.proof,
            operator_signature: (!value.operator_signature.is_empty())
                .then_some(value.operator_signature),
        })
    }
}

impl From<ZKDCAPRegisterEnclaveKeyMessage> for RawZKDCAPRegisterEnclaveKeyMessage {
    fn from(value: ZKDCAPRegisterEnclaveKeyMessage) -> Self {
        RawZKDCAPRegisterEnclaveKeyMessage {
            commit: value.commit.to_bytes(),
            proof: value.proof,
            operator_signature: value.operator_signature.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UpdateClientMessage {
    pub signatures: Vec<Vec<u8>>,
    pub proxy_message: ProxyMessage,
}

impl Protobuf<RawUpdateClientMessage> for UpdateClientMessage {}

impl TryFrom<RawUpdateClientMessage> for UpdateClientMessage {
    type Error = Error;
    fn try_from(value: RawUpdateClientMessage) -> Result<Self, Self::Error> {
        Ok(UpdateClientMessage {
            signatures: value.signatures,
            proxy_message: ProxyMessage::from_bytes(&value.proxy_message)?,
        })
    }
}

impl From<UpdateClientMessage> for RawUpdateClientMessage {
    fn from(value: UpdateClientMessage) -> Self {
        RawUpdateClientMessage {
            proxy_message: Into::<ProxyMessage>::into(value.proxy_message).to_bytes(),
            signatures: value.signatures,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UpdateOperatorsMessage {
    pub nonce: u64,
    pub new_operators: Vec<Address>,
    pub new_operators_threshold_numerator: u64,
    pub new_operators_threshold_denominator: u64,
    pub signatures: Vec<Vec<u8>>,
}

impl Protobuf<RawUpdateOperatorsMessage> for UpdateOperatorsMessage {}

impl TryFrom<RawUpdateOperatorsMessage> for UpdateOperatorsMessage {
    type Error = Error;
    fn try_from(value: RawUpdateOperatorsMessage) -> Result<Self, Self::Error> {
        Ok(UpdateOperatorsMessage {
            nonce: value.nonce,
            new_operators: value
                .new_operators
                .iter()
                .map(|op| Address::try_from(op.as_slice()))
                .collect::<Result<_, _>>()?,
            new_operators_threshold_numerator: value.new_operators_threshold_numerator,
            new_operators_threshold_denominator: value.new_operators_threshold_denominator,
            signatures: value.signatures,
        })
    }
}

impl From<UpdateOperatorsMessage> for RawUpdateOperatorsMessage {
    fn from(value: UpdateOperatorsMessage) -> Self {
        RawUpdateOperatorsMessage {
            nonce: value.nonce,
            new_operators: value
                .new_operators
                .into_iter()
                .map(|op| op.to_vec())
                .collect(),
            new_operators_threshold_numerator: value.new_operators_threshold_numerator,
            new_operators_threshold_denominator: value.new_operators_threshold_denominator,
            signatures: value.signatures,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CommitmentProofs {
    pub message: Vec<u8>,
    pub signatures: Vec<Vec<u8>>,
}

impl CommitmentProofs {
    pub fn message(&self) -> Result<ProxyMessage, CommitmentError> {
        ProxyMessage::from_bytes(&self.message)
    }
}

sol! {
    struct EthABICommitmentProofs {
        bytes message;
        bytes[] signatures;
    }
}

impl EthABIEncoder for CommitmentProofs {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABICommitmentProofs>::into(self).abi_encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, CommitmentError> {
        Ok(EthABICommitmentProofs::abi_decode(bz, true).unwrap().into())
    }
}

impl From<EthABICommitmentProofs> for CommitmentProofs {
    fn from(value: EthABICommitmentProofs) -> Self {
        Self {
            message: value.message.into(),
            signatures: value.signatures.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CommitmentProofs> for EthABICommitmentProofs {
    fn from(value: CommitmentProofs) -> Self {
        Self {
            message: value.message.into(),
            signatures: value.signatures.into_iter().map(Into::into).collect(),
        }
    }
}

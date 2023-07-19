use crate::errors::Error;
use crate::prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use commitments::{StateID, UpdateClientCommitment};
use crypto::Address;
use ibc_proto::protobuf::Protobuf;
use lcp_proto::ibc::lightclients::lcp::v1::{
    RegisterEnclaveKeyHeader as RawRegisterEnclaveKeyHeader,
    UpdateClientHeader as RawUpdateClientHeader,
};
use lcp_types::{Any, Height, Time};
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};
use validation_context::ValidationParams;

pub const LCP_HEADER_ACTIVATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.Header.Activate";
pub const LCP_HEADER_REGISTER_ENCLAVE_KEY_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.Header.RegisterEnclaveKey";
pub const LCP_HEADER_UPDATE_CLIENT_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.Header.UpdateClient";

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Header {
    RegisterEnclaveKey(RegisterEnclaveKeyHeader),
    UpdateClient(UpdateClientHeader),
}

impl Protobuf<ProtoAny> for Header {}

impl TryFrom<ProtoAny> for Header {
    type Error = Error;

    fn try_from(raw: ProtoAny) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            LCP_HEADER_UPDATE_CLIENT_TYPE_URL => Ok(Header::UpdateClient(
                UpdateClientHeader::decode_vec(&raw.value).map_err(Error::ibc_proto)?,
            )),
            _ => Err(Error::unexpected_header_type(raw.type_url)),
        }
    }
}

impl From<Header> for ProtoAny {
    fn from(value: Header) -> Self {
        match value {
            Header::UpdateClient(h) => ProtoAny {
                type_url: LCP_HEADER_UPDATE_CLIENT_TYPE_URL.to_string(),
                value: h.encode_vec().unwrap(),
            },
            _ => unimplemented!(),
        }
    }
}

impl TryFrom<Any> for Header {
    type Error = Error;

    fn try_from(any: Any) -> Result<Self, Self::Error> {
        TryFrom::<ProtoAny>::try_from(any.into())
    }
}

impl From<Header> for Any {
    fn from(value: Header) -> Self {
        ProtoAny::from(value).try_into().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterEnclaveKeyHeader(pub EndorsedAttestationVerificationReport);

impl Protobuf<RawRegisterEnclaveKeyHeader> for RegisterEnclaveKeyHeader {}

impl TryFrom<RawRegisterEnclaveKeyHeader> for RegisterEnclaveKeyHeader {
    type Error = Error;
    fn try_from(value: RawRegisterEnclaveKeyHeader) -> Result<Self, Self::Error> {
        Ok(RegisterEnclaveKeyHeader(
            EndorsedAttestationVerificationReport {
                avr: value.report,
                signature: value.signature,
                signing_cert: value.signing_cert,
            },
        ))
    }
}

impl From<RegisterEnclaveKeyHeader> for RawRegisterEnclaveKeyHeader {
    fn from(value: RegisterEnclaveKeyHeader) -> Self {
        RawRegisterEnclaveKeyHeader {
            report: (&value.0.avr).try_into().unwrap(),
            signature: value.0.signature,
            signing_cert: value.0.signing_cert,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UpdateClientHeader {
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
    pub commitment: UpdateClientCommitment,
}

impl Protobuf<RawUpdateClientHeader> for UpdateClientHeader {}

impl TryFrom<RawUpdateClientHeader> for UpdateClientHeader {
    type Error = Error;
    fn try_from(value: RawUpdateClientHeader) -> Result<Self, Self::Error> {
        Ok(UpdateClientHeader {
            signer: value.signer,
            signature: value.signature,
            commitment: UpdateClientCommitment::from_bytes(&value.commitment).unwrap(),
            commitment_bytes: value.commitment,
        })
    }
}

impl From<UpdateClientHeader> for RawUpdateClientHeader {
    fn from(value: UpdateClientHeader) -> Self {
        RawUpdateClientHeader {
            commitment: value.commitment.to_vec(),
            signer: value.signer,
            signature: value.signature,
        }
    }
}

impl Commitment for UpdateClientHeader {
    fn signer(&self) -> Address {
        self.signer.as_slice().try_into().unwrap()
    }

    fn commitment(&self) -> &UpdateClientCommitment {
        &self.commitment
    }
}

pub trait Commitment {
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

    fn validation_params(&self) -> &ValidationParams {
        &self.commitment().validation_params
    }
}

impl Header {
    pub fn get_height(&self) -> Option<Height> {
        match self {
            Header::UpdateClient(h) => Some(h.height()),
            _ => None,
        }
    }

    pub fn get_timestamp(&self) -> Option<Time> {
        match self {
            Header::UpdateClient(h) => Some(h.timestamp()),
            _ => None,
        }
    }
}

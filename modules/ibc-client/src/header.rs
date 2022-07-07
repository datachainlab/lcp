#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{crypto::Address, report::AttestationVerificationReport};
use commitments::{StateID, UpdateClientCommitment};
use ibc::core::ics02_client::{
    client_type::ClientType, error::Error, header::AnyHeader, height::Height as ICS02Height,
};
use ibc::timestamp::Timestamp;
use lcp_proto::ibc::lightclients::lcp::v1::UpdateClientHeader as RawUpdateClientHeader;
use lcp_types::{Any, Height};
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};
use tendermint_proto::Protobuf;
use validation_context::ValidationParams;

pub const LCP_HEADER_ACTIVATE_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.Header.Activate";
pub const LCP_HEADER_REGISTER_ENCLAVE_KEY_TYPE_URL: &str =
    "/ibc.lightclients.lcp.v1.Header.RegisterEnclaveKey";
pub const LCP_HEADER_UPDATE_CLIENT_TYPE_URL: &str = "/ibc.lightclients.lcp.v1.Header.UpdateClient";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Header {
    Activate(ActivateHeader),
    RegisterEnclaveKey(RegisterEnclaveKeyHeader),
    UpdateClient(UpdateClientHeader),
}

impl Protobuf<ProtoAny> for Header {}

impl TryFrom<ProtoAny> for Header {
    type Error = Error;

    fn try_from(raw: ProtoAny) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            "" => Err(Error::empty_client_state_response()),
            LCP_HEADER_UPDATE_CLIENT_TYPE_URL => Ok(Header::UpdateClient(
                UpdateClientHeader::decode_vec(&raw.value).map_err(Error::invalid_raw_header)?,
            )),
            _ => Err(Error::unknown_header_type(raw.type_url)),
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

impl From<Header> for Any {
    fn from(value: Header) -> Self {
        ProtoAny::from(value).into()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ActivateHeader {
    pub initial_state_bytes: Vec<u8>,
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
    pub commitment: UpdateClientCommitment,
}

impl Commitment for ActivateHeader {
    fn signer(&self) -> Address {
        self.signer.as_slice().into()
    }

    fn commitment(&self) -> &UpdateClientCommitment {
        &self.commitment
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterEnclaveKeyHeader(pub AttestationVerificationReport);

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
        self.signer.as_slice().into()
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

    fn timestamp_as_u128(&self) -> u128 {
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

    pub fn get_timestamp(&self) -> Option<Timestamp> {
        match self {
            Header::UpdateClient(h) => {
                Some(Timestamp::from_nanoseconds(h.timestamp_as_u128() as u64).unwrap())
            }
            _ => None,
        }
    }
}

impl ibc::core::ics02_client::header::Header for Header {
    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn height(&self) -> ICS02Height {
        self.get_height().unwrap().try_into().unwrap()
    }

    fn timestamp(&self) -> Timestamp {
        self.get_timestamp().unwrap()
    }

    fn wrap_any(self) -> AnyHeader {
        // NOTE: AnyHeader is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }
}

use crate::prelude::*;
use core::ops::Deref;
use ibc::core::ics02_client::client_consensus::AnyConsensusState;
use ibc::core::ics02_client::client_state::AnyClientState;
use ibc::core::ics02_client::error::Error;
use ibc::core::ics02_client::header::AnyHeader;
use ibc_proto::google::protobuf::Any as IBCAny;
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};
use tendermint_proto::Protobuf;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Any(#[serde(with = "ProtoAnyDef")] ProtoAny);

impl Any {
    pub fn new<A: Into<ProtoAny>>(any: A) -> Self {
        Self(any.into())
    }
    pub fn to_proto(self) -> ProtoAny {
        self.into()
    }
    pub fn to_ibc(self) -> IBCAny {
        self.into()
    }
}

impl Deref for Any {
    type Target = ProtoAny;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ProtoAny> for Any {
    fn from(v: ProtoAny) -> Self {
        Self(v)
    }
}

impl From<Any> for ProtoAny {
    fn from(v: Any) -> Self {
        v.0
    }
}

impl From<Any> for IBCAny {
    fn from(v: Any) -> Self {
        IBCAny {
            type_url: v.0.type_url,
            value: v.0.value,
        }
    }
}

impl From<IBCAny> for Any {
    fn from(v: IBCAny) -> Self {
        Any(ProtoAny {
            type_url: v.type_url,
            value: v.value,
        })
    }
}

impl TryFrom<Any> for AnyClientState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        IBCAny::from(value).try_into()
    }
}

impl From<AnyClientState> for Any {
    fn from(v: AnyClientState) -> Self {
        IBCAny::from(v).into()
    }
}

impl TryFrom<Any> for AnyConsensusState {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        IBCAny::from(value).try_into()
    }
}

impl From<AnyConsensusState> for Any {
    fn from(v: AnyConsensusState) -> Self {
        IBCAny::from(v).into()
    }
}

impl TryFrom<Any> for AnyHeader {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        IBCAny::from(value).try_into()
    }
}

impl From<AnyHeader> for Any {
    fn from(v: AnyHeader) -> Self {
        IBCAny::from(v).into()
    }
}

impl TryFrom<Vec<u8>> for Any {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Any::decode_vec(&value).unwrap())
    }
}

impl Protobuf<ProtoAny> for Any {}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(remote = "ProtoAny")]
pub struct ProtoAnyDef {
    pub type_url: String,
    pub value: Vec<u8>,
}

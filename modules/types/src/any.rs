use crate::prelude::*;
use core::ops::Deref;
use ibc::core::ics02_client::error::ClientError as Error;
use ibc_proto::google::protobuf::Any as IBCAny;
use ibc_proto::protobuf::Protobuf;
use prost_types::Any as ProtoAny;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Any(#[serde(with = "ProtoAnyDef")] ProtoAny);

impl Any {
    pub fn new(type_url: String, value: Vec<u8>) -> Self {
        Self(ProtoAny { type_url, value })
    }

    pub fn from_any<A: Into<ProtoAny>>(any: A) -> Self {
        Self(any.into())
    }

    pub fn to_proto(self) -> ProtoAny {
        self.into()
    }
}

impl Deref for Any {
    type Target = ProtoAny;

    fn deref(&self) -> &Self::Target {
        &self.0
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

impl TryFrom<Vec<u8>> for Any {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Any::decode_vec(&value).unwrap())
    }
}

impl TryFrom<ProtoAny> for Any {
    type Error = Error;

    fn try_from(value: ProtoAny) -> Result<Self, Self::Error> {
        Ok(Self::from_any(value))
    }
}

impl Protobuf<ProtoAny> for Any {}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(remote = "ProtoAny")]
pub struct ProtoAnyDef {
    pub type_url: String,
    pub value: Vec<u8>,
}

use crate::errors::TypeError;
use crate::prelude::*;
use core::ops::Deref;
use lcp_proto::{google::protobuf::Any as ProtoAny, protobuf::Protobuf};
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

impl From<ProtoAny> for Any {
    fn from(v: ProtoAny) -> Self {
        Any(ProtoAny {
            type_url: v.type_url,
            value: v.value,
        })
    }
}

impl TryFrom<Vec<u8>> for Any {
    type Error = TypeError;
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

impl prost::Message for Any {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
        Self: Sized,
    {
        self.0.encode_raw(buf)
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut B,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError>
    where
        B: prost::bytes::Buf,
        Self: Sized,
    {
        self.0.merge_field(tag, wire_type, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }

    fn clear(&mut self) {
        self.0.clear()
    }
}

use crate::errors::TypeError;
use crate::prelude::*;
use core::ops::Deref;
use lcp_proto::{google::protobuf::Any as ProtoAny, Protobuf};
use prost::bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Any(#[serde(with = "ProtoAnyDef")] ProtoAny);

impl Any {
    pub fn new(type_url: String, value: Vec<u8>) -> Self {
        Self(ProtoAny { type_url, value })
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
        Ok(Any::decode_vec(&value)?)
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
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        self.0.encode_raw(buf)
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError>
    where
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

#[cfg(feature = "ibc")]
impl From<ibc_primitives::proto::Any> for Any {
    fn from(value: ibc_primitives::proto::Any) -> Self {
        ProtoAny {
            type_url: value.type_url,
            value: value.value,
        }
        .into()
    }
}

#[cfg(feature = "ibc")]
impl From<Any> for ibc_primitives::proto::Any {
    fn from(v: Any) -> Self {
        ibc_primitives::proto::Any {
            type_url: v.type_url.clone(),
            value: v.value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use prost::Message;

    proptest! {
        #[test]
        fn test_encoding_compatibility_with_proto_any(type_url: String, value: Vec<u8>) {
            let any1 = Any::new(type_url, value);
            let bz = any1.encode_to_vec();
            let any2 = ProtoAny{
                type_url: any1.type_url.clone(),
                value: any1.value.clone(),
            };
            let bz2 = any2.encode_to_vec();
            assert_eq!(bz, bz2);

            let any3 = Any::decode_vec(&bz).unwrap();
            assert_eq!(any1, any3);
        }
    }
}

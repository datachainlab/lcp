use crate::errors::TypeError;
use crate::prelude::*;
use core::fmt::{Debug, Formatter, Result as DebugResult};
use core::ops::Deref;
use lcp_proto::{google::protobuf::Any as ProtoAny, protobuf::Protobuf};
use serde::{Deserialize, Serialize};

const MAX_VALUE_LENGTH_FOR_DEBUG: usize = 4096;

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
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

impl Debug for Any {
    fn fmt(&self, f: &mut Formatter<'_>) -> DebugResult {
        let mut debug = f.debug_struct("Any");
        debug.field("type_url", &self.type_url);
        if self.value.len() > MAX_VALUE_LENGTH_FOR_DEBUG {
            debug.field(
                "value",
                &format_args!(
                    "{:?} … ({} bytes total)",
                    &self.value[..MAX_VALUE_LENGTH_FOR_DEBUG],
                    self.value.len()
                ),
            );
        } else {
            debug.field("value", &self.value);
        }
        debug.finish()
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

    #[test]
    fn test_debug_any() {
        let type_url = "type_url".to_string();
        let base = Any::new(type_url.clone(), [0u8; MAX_VALUE_LENGTH_FOR_DEBUG].to_vec());
        let base_str = format!("{:?}", base);
        {
            let value = [0u8; MAX_VALUE_LENGTH_FOR_DEBUG + 1].to_vec();
            let value_str = format!("{:?}", Any::new(type_url.clone(), value));
            assert_ne!(value_str, base_str);
            assert!(
                value_str.contains(&format!("({} bytes total)", MAX_VALUE_LENGTH_FOR_DEBUG + 1))
            );
        }
        {
            let value = [0u8; MAX_VALUE_LENGTH_FOR_DEBUG].to_vec();
            assert_eq!(base_str, format!("{:?}", Any::new(type_url.clone(), value)));
        }
        {
            let value = [0u8; MAX_VALUE_LENGTH_FOR_DEBUG - 1].to_vec();
            let value_str = format!("{:?}", Any::new(type_url.clone(), value));
            assert_ne!(value_str, base_str);
            assert!(!value_str.contains("bytes total"));
        }
    }
}

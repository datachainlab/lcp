use crate::errors::Result;
use prost::Message;
use prost_types::Any;

pub fn parse_bytes_into_any(bz: &[u8]) -> Result<Any> {
    let any = Any::decode(bz).unwrap();
    Ok(any)
}

use crate::prelude::*;
use alloc::string::ToString;
use core::marker::PhantomData;
use serde::Deserialize;
use serde_with::{DeserializeAs, SerializeAs};
use sgx_types::marker::ContiguousMemory;

pub struct BytesTransmuter<T>(PhantomData<T>);

impl<T> SerializeAs<T> for BytesTransmuter<T>
where
    [(); core::mem::size_of::<T>()]:,
    T: ContiguousMemory,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&unsafe { serialize_bytes(source) })
    }
}

impl<'de, T> DeserializeAs<'de, T> for BytesTransmuter<T>
where
    [(); core::mem::size_of::<T>()]:,
    T: ContiguousMemory,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bz = <&[u8]>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
        unsafe { deserialize_bytes(bz) }.map_err(|(len, size)| {
            serde::de::Error::invalid_length(len, &size.to_string().as_str())
        })
    }
}

pub unsafe fn serialize_bytes<T>(source: &T) -> [u8; core::mem::size_of::<T>()]
where
    [(); core::mem::size_of::<T>()]:,
    T: ContiguousMemory,
{
    unsafe { core::mem::transmute_copy::<_, [u8; core::mem::size_of::<T>()]>(source) }
}

pub unsafe fn deserialize_bytes<T>(bz: &[u8]) -> Result<T, (usize, usize)>
where
    [(); core::mem::size_of::<T>()]:,
    T: ContiguousMemory,
{
    let mut array = [0; core::mem::size_of::<T>()];
    if bz.len() == array.len() {
        array.copy_from_slice(bz);
        Ok(unsafe { core::mem::transmute_copy(&array) })
    } else {
        Err((bz.len(), array.len()))
    }
}

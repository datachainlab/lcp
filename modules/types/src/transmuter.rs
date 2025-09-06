use crate::prelude::*;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::marker::PhantomData;
use serde::Deserialize;
use serde_with::{DeserializeAs, SerializeAs};
use sgx_types::marker::ContiguousMemory;

pub struct BytesTransmuter<T>(PhantomData<T>);

impl<T> SerializeAs<T> for BytesTransmuter<T>
where
    T: ContiguousMemory,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&serialize_bytes(source))
    }
}

impl<'de, T> DeserializeAs<'de, T> for BytesTransmuter<T>
where
    T: ContiguousMemory,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bz = <&[u8]>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
        deserialize_bytes(bz).map_err(|(len, size)| {
            serde::de::Error::invalid_length(len, &size.to_string().as_str())
        })
    }
}

pub fn serialize_bytes<T>(source: &T) -> Vec<u8>
where
    T: ContiguousMemory,
{
    let size = core::mem::size_of::<T>();
    let ptr = source as *const T as *const u8;
    let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
    slice.to_vec()
}

pub fn deserialize_bytes<T>(bz: &[u8]) -> Result<T, (usize, usize)>
where
    T: ContiguousMemory,
{
    let expected_size = core::mem::size_of::<T>();
    if bz.len() == expected_size {
        let mut value = core::mem::MaybeUninit::<T>::uninit();
        let ptr = value.as_mut_ptr() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, expected_size) };
        slice.copy_from_slice(bz);
        Ok(unsafe { value.assume_init() })
    } else {
        Err((bz.len(), expected_size))
    }
}

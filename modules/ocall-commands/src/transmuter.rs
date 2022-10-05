use alloc::string::ToString;
use core::marker::PhantomData;
use serde::Deserialize;
use serde_with::{DeserializeAs, SerializeAs};

pub(crate) struct BytesTransmuter<T>(PhantomData<T>);

impl<T> SerializeAs<T> for BytesTransmuter<T>
where
    [(); core::mem::size_of::<T>()]:,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&unsafe {
            core::mem::transmute_copy::<_, [u8; core::mem::size_of::<T>()]>(source)
        })
    }
}

impl<'de, T> DeserializeAs<'de, T> for BytesTransmuter<T>
where
    [(); core::mem::size_of::<T>()]:,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bz = <&[u8]>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
        let mut array = [0; core::mem::size_of::<T>()];
        if bz.len() == array.len() {
            array.copy_from_slice(bz);
            Ok(unsafe { core::mem::transmute_copy(&array) })
        } else {
            Err(serde::de::Error::invalid_length(
                bz.len(),
                &array.len().to_string().as_str(),
            ))
        }
    }
}

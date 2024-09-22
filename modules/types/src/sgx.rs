use crate::{prelude::*, TypeError};
use core::fmt::Display;
use core::ops::Deref;
use sgx_types::{sgx_measurement_t, SGX_HASH_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mrenclave(pub [u8; SGX_HASH_SIZE]);

impl Deref for Mrenclave {
    type Target = [u8; SGX_HASH_SIZE];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Mrenclave {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

impl From<sgx_measurement_t> for Mrenclave {
    fn from(measurement: sgx_measurement_t) -> Self {
        Self(measurement.m)
    }
}

impl From<[u8; SGX_HASH_SIZE]> for Mrenclave {
    fn from(bytes: [u8; SGX_HASH_SIZE]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<Vec<u8>> for Mrenclave {
    type Error = TypeError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != SGX_HASH_SIZE {
            return Err(TypeError::mrenclave_bytes_conversion(value));
        }
        let mut bytes = [0u8; SGX_HASH_SIZE];
        bytes.copy_from_slice(&value);
        Ok(Self(bytes))
    }
}

impl Mrenclave {
    pub fn to_hex_string(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
    pub fn from_hex_string(s: &str) -> Result<Self, TypeError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; SGX_HASH_SIZE];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

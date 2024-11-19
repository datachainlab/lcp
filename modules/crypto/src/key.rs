use crate::prelude::*;
use crate::{Error, Keccak256, Signer, Verifier};
use alloc::fmt;
use core::fmt::Display;
use core::sync::atomic;
use libsecp256k1::{
    curve::Scalar,
    util::{COMPRESSED_PUBLIC_KEY_SIZE, SECRET_KEY_SIZE},
    Message, PublicKey, PublicKeyFormat, RecoveryId, SecretKey, Signature,
};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sgx_types::sgx_sealed_data_t;
use tiny_keccak::Keccak;
use zeroize::Zeroizing;

pub struct EnclaveKey {
    pub(crate) secret_key: SecretKey,
}

impl EnclaveKey {
    #[cfg(any(feature = "std", feature = "sgx"))]
    pub fn new() -> Result<Self, Error> {
        #[cfg(feature = "sgx")]
        use crate::sgx::rand::rand_slice;

        #[cfg(feature = "std")]
        fn rand_slice(bz: &mut [u8]) -> Result<(), Error> {
            use rand::{thread_rng, Rng};
            thread_rng().fill(bz);
            Ok(())
        }

        let secret_key = loop {
            let mut ret = [0u8; SECRET_KEY_SIZE];
            rand_slice(ret.as_mut())?;

            if let Ok(key) = SecretKey::parse(&ret) {
                break key;
            }
        };
        Ok(Self { secret_key })
    }

    pub fn get_privkey(self) -> Zeroizing<[u8; SECRET_KEY_SIZE]> {
        Zeroizing::new(self.secret_key.serialize())
    }

    pub fn get_pubkey(&self) -> EnclavePublicKey {
        EnclavePublicKey(PublicKey::from_secret_key(&self.secret_key))
    }
}

impl Drop for EnclaveKey {
    fn drop(&mut self) {
        self.secret_key.clear();
        // Use fences to prevent accesses from being reordered before this
        // point, which should hopefully help ensure that all accessors
        // see zeroes after this point.
        atomic::compiler_fence(atomic::Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclavePublicKey(PublicKey);

impl Serialize for EnclavePublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Vec::<u8>::serialize(self.as_array().to_vec().as_ref(), serializer)
    }
}

impl<'de> serde::Deserialize<'de> for EnclavePublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = EnclavePublicKey;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("compressed public key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                EnclavePublicKey::try_from(v).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

impl TryFrom<&[u8]> for EnclavePublicKey {
    type Error = Error;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(
            PublicKey::parse_slice(v, Some(PublicKeyFormat::Compressed))
                .map_err(Error::secp256k1)?,
        ))
    }
}

impl TryFrom<EnclavePublicKey> for Vec<u8> {
    type Error = Error;
    fn try_from(value: EnclavePublicKey) -> Result<Self, Self::Error> {
        Ok(value.as_array().to_vec())
    }
}

impl EnclavePublicKey {
    pub fn as_array(&self) -> [u8; COMPRESSED_PUBLIC_KEY_SIZE] {
        self.0.serialize_compressed()
    }

    pub fn as_address(&self) -> Address {
        let pubkey = &self.0.serialize()[1..];
        let mut addr: Address = Default::default();
        addr.0.copy_from_slice(&keccak256(pubkey)[12..]);
        addr
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
    pub fn to_hex_string(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
    pub fn from_hex_string(s: &str) -> Result<Self, Error> {
        let bz = hex::decode(s.strip_prefix("0x").unwrap_or(s))?;
        Address::try_from(bz.as_slice())
    }
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 20]
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.to_hex_string().as_str())
    }
}

impl From<Address> for Vec<u8> {
    fn from(value: Address) -> Self {
        value.0.to_vec()
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 20 {
            Err(Error::invalid_address_length(value.len()))
        } else {
            let mut addr = Address::default();
            addr.0.copy_from_slice(value);
            Ok(addr)
        }
    }
}

impl Signer for EnclaveKey {
    fn sign(&self, bz: &[u8]) -> Result<Vec<u8>, Error> {
        let mut s = Scalar::default();
        let _ = s.set_b32(&bz.keccak256());
        let (sig, rid) = libsecp256k1::sign(&Message(s), &self.secret_key);
        let mut ret = vec![0; 65];
        ret[..64].copy_from_slice(&sig.serialize());
        ret[64] = rid.serialize();
        Ok(ret)
    }
    fn pubkey(&self) -> Result<EnclavePublicKey, Error> {
        Ok(self.get_pubkey())
    }
}

impl Verifier for EnclavePublicKey {
    fn verify(&self, msg: &[u8], signature: &[u8]) -> Result<(), Error> {
        let signer = verify_signature(msg, signature)?;
        if self.eq(&signer) {
            Ok(())
        } else {
            Err(Error::unexpected_signer(self.clone(), signer))
        }
    }
}

pub fn verify_signature(sign_bytes: &[u8], signature: &[u8]) -> Result<EnclavePublicKey, Error> {
    if signature.len() != 65 {
        return Err(Error::invalid_signature_length(signature.len()));
    }

    let sign_hash = keccak256(sign_bytes);
    let mut s = Scalar::default();
    let _ = s.set_b32(&sign_hash);

    let sig = Signature::parse_standard_slice(&signature[..64]).map_err(Error::secp256k1)?;
    let rid = RecoveryId::parse(signature[64]).map_err(Error::secp256k1)?;
    let signer = libsecp256k1::recover(&Message(s), &sig, &rid).map_err(Error::secp256k1)?;
    Ok(EnclavePublicKey(signer))
}

pub fn verify_signature_address(sign_bytes: &[u8], signature: &[u8]) -> Result<Address, Error> {
    Ok(verify_signature(sign_bytes, signature)?.as_address())
}

fn keccak256(bz: &[u8]) -> [u8; 32] {
    let mut keccak = Keccak::new_keccak256();
    let mut result = [0u8; 32];
    keccak.update(bz);
    keccak.finalize(result.as_mut());
    result
}

pub const SEALED_DATA_32_SIZE: u32 = calc_raw_sealed_data_size(0, 32);
pub const SEALED_DATA_32_USIZE: usize = safe_u32_to_usize(SEALED_DATA_32_SIZE);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealedEnclaveKey(#[serde(with = "BigArray")] pub(crate) [u8; SEALED_DATA_32_USIZE]);

impl SealedEnclaveKey {
    pub fn new(sealed_ek: [u8; SEALED_DATA_32_USIZE]) -> Self {
        Self(sealed_ek)
    }

    pub fn new_from_bytes(bz: &[u8]) -> Result<Self, Error> {
        if bz.len() != SEALED_DATA_32_USIZE {
            return Err(Error::invalid_sealed_enclave_key("".to_owned()));
        }
        let mut data = [0; SEALED_DATA_32_USIZE];
        data.copy_from_slice(bz);
        Ok(Self::new(data))
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

// modified copy from sgx_tseal/src/internal.rs
const fn calc_raw_sealed_data_size(add_mac_txt_size: u32, encrypt_txt_size: u32) -> u32 {
    let max = u32::MAX;
    let sealed_data_size = core::mem::size_of::<sgx_sealed_data_t>() as u32;

    if add_mac_txt_size > max - encrypt_txt_size {
        return max;
    }
    let payload_size: u32 = add_mac_txt_size + encrypt_txt_size;
    if payload_size > max - sealed_data_size {
        return max;
    }
    sealed_data_size + payload_size
}

#[allow(clippy::assertions_on_constants)]
const fn safe_u32_to_usize(v: u32) -> usize {
    assert!(usize::BITS >= 32);
    v as usize
}

pub struct NopSigner;

impl Signer for NopSigner {
    fn pubkey(&self) -> Result<EnclavePublicKey, Error> {
        Err(Error::nop_signer())
    }
    fn sign(&self, _: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::nop_signer())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeroize_enclave_key() {
        let ptr = {
            let ek = EnclaveKey::new().unwrap();
            let ptr = &ek.secret_key as *const SecretKey as *const u8;
            let slice = unsafe { core::slice::from_raw_parts(ptr, SECRET_KEY_SIZE) };
            assert_ne!(slice, &[0u8; SECRET_KEY_SIZE]);
            ptr
        };
        let slice = unsafe { core::slice::from_raw_parts(ptr, SECRET_KEY_SIZE) };
        assert_eq!(slice, &[0u8; SECRET_KEY_SIZE]);
    }
}

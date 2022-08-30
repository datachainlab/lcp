#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{
    traits::{Keccak256, SealedKey},
    CryptoError as Error, Signer, Verifier,
};
use anyhow::anyhow;
use secp256k1::curve::Scalar;
use secp256k1::{
    util::{COMPRESSED_PUBLIC_KEY_SIZE, SECRET_KEY_SIZE},
    Message, PublicKey, RecoveryId, SecretKey, Signature,
};
use serde::{Deserialize, Serialize};
use sgx_types::sgx_report_data_t;
use std::format;
use std::io::{Read, Write};
use std::vec;
use std::vec::Vec;
use tiny_keccak::Keccak;

#[cfg(feature = "sgx")]
use crate::sgx::rand::rand_slice;
#[cfg(not(feature = "sgx"))]
use std::fs::File;
#[cfg(feature = "sgx")]
use std::sgxfs::SgxFile as File;
#[cfg(not(feature = "sgx"))]
fn rand_slice(bz: &mut [u8]) -> Result<(), Error> {
    use rand::{thread_rng, Rng};
    thread_rng().fill(bz);
    Ok(())
}

#[derive(Default)]
pub struct EnclaveKey {
    secret_key: SecretKey,
}

impl EnclaveKey {
    pub fn new() -> Result<Self, Error> {
        let secret_key = loop {
            let mut ret = [0u8; SECRET_KEY_SIZE];
            rand_slice(ret.as_mut())?;

            if let Ok(key) = SecretKey::parse(&ret) {
                break key;
            }
        };
        Ok(Self { secret_key })
    }

    pub fn get_privkey(&self) -> [u8; SECRET_KEY_SIZE] {
        self.secret_key.serialize()
    }

    pub fn get_pubkey(&self) -> EnclavePublicKey {
        EnclavePublicKey(PublicKey::from_secret_key(&self.secret_key))
    }

    pub fn sign(&self, bz: &[u8]) -> Result<Vec<u8>, Error> {
        self.sign_hash(&bz.keccak256())
    }

    fn sign_hash(&self, bz: &[u8; 32]) -> Result<Vec<u8>, Error> {
        let mut s = Scalar::default();
        let _ = s.set_b32(&bz);
        let (sig, rid) = secp256k1::sign(&Message(s), &self.secret_key);
        let mut ret = vec![0; 65];
        ret[..64].copy_from_slice(&sig.serialize());
        ret[64] = rid.serialize();
        Ok(ret)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclavePublicKey(PublicKey);

impl TryFrom<&[u8]> for EnclavePublicKey {
    type Error = Error;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(
            PublicKey::parse_slice(v, None).map_err(Error::Secp256k1Error)?,
        ))
    }
}

impl EnclavePublicKey {
    pub fn as_bytes(&self) -> [u8; COMPRESSED_PUBLIC_KEY_SIZE] {
        self.0.serialize_compressed()
    }

    pub fn as_report_data(&self) -> sgx_report_data_t {
        let mut report_data = sgx_report_data_t::default();
        report_data.d[..20].copy_from_slice(Address::from(self).as_ref());
        report_data
    }

    pub fn verify(&self, msg: &[u8], signature: &[u8]) -> Result<(), Error> {
        let signer = verify_signature(msg, signature)?;
        if self.eq(&signer) {
            Ok(())
        } else {
            Err(Error::VerificationError(format!(
                "unexpected signer: {:?}",
                signer
            )))
        }
    }
}

impl From<&EnclavePublicKey> for Address {
    fn from(v: &EnclavePublicKey) -> Self {
        let pubkey = &v.0.serialize_compressed()[1..];
        let mut res: Address = Default::default();
        res.0.copy_from_slice(&keccak256(pubkey)[12..]);
        res
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]);

impl From<&[u8]> for Address {
    fn from(v: &[u8]) -> Self {
        assert!(v.len() == 20);
        let mut addr = Address::default();
        addr.0.copy_from_slice(v);
        addr
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Address> for Vec<u8> {
    fn from(v: Address) -> Self {
        v.as_ref().to_vec()
    }
}

impl Signer for EnclaveKey {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        self.sign(msg)
    }

    fn use_verifier(&self, f: &mut dyn FnMut(&dyn Verifier)) {
        f(&self.get_pubkey());
    }
}

impl Verifier for EnclavePublicKey {
    fn get_pubkey(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn get_address(&self) -> Vec<u8> {
        Address::from(self).into()
    }

    fn verify(&self, msg: &[u8], signature: &[u8]) -> Result<(), Error> {
        self.verify(msg, signature)
    }
}

impl SealedKey for EnclaveKey {
    fn seal(&self, filepath: &str) -> Result<(), Error> {
        // Files are automatically closed when they go out of scope.
        seal(&self.get_privkey(), filepath)
    }

    fn unseal(filepath: &str) -> Result<Self, Error> {
        let secret_key = open(filepath)?;
        Ok(Self { secret_key })
    }
}

fn seal(data: &[u8; 32], filepath: &str) -> Result<(), Error> {
    let mut file = File::create(filepath)
        .map_err(|e| Error::FailedSeal(e, format!("error creating file: {}", filepath)))?;
    file.write_all(data)
        .map_err(|e| Error::FailedSeal(e, format!("error writing to path: {}", filepath)))
}

fn open(filepath: &str) -> Result<SecretKey, Error> {
    let mut file = File::open(filepath)
        .map_err(|e| Error::FailedUnseal(e, format!("failed to open file: {}", filepath)))?;

    let mut buf = [0u8; SECRET_KEY_SIZE];
    let n = file
        .read(buf.as_mut())
        .map_err(|e| Error::FailedUnseal(e, format!("failed to read file: {}", filepath)))?;

    if n < SECRET_KEY_SIZE {
        return Err(Error::OtherError(anyhow!(
            "[Enclave] Dramatic read from {} ended prematurely (n = {} < SECRET_KEY_SIZE = {})",
            filepath,
            n,
            SECRET_KEY_SIZE
        )));
    }
    Ok(SecretKey::parse(&buf).unwrap())
}

pub fn verify_signature(sign_bytes: &[u8], signature: &[u8]) -> Result<EnclavePublicKey, Error> {
    assert!(signature.len() == 65);

    let sign_hash = keccak256(sign_bytes);
    let mut s = Scalar::default();
    let _ = s.set_b32(&sign_hash);

    let sig = Signature::parse_slice(&signature[..64])?;
    let rid = RecoveryId::parse(signature[64])?;
    let signer = secp256k1::recover(&Message(s), &sig, &rid)?;
    Ok(EnclavePublicKey(signer))
}

pub fn verify_signature_address(sign_bytes: &[u8], signature: &[u8]) -> Result<Address, Error> {
    let pub_key = verify_signature(sign_bytes, signature)?;
    Ok((&pub_key).into())
}

fn keccak256(bz: &[u8]) -> [u8; 32] {
    let mut keccak = Keccak::new_keccak256();
    let mut result = [0u8; 32];
    keccak.update(bz);
    keccak.finalize(result.as_mut());
    result
}

use crate::traits::Keccak256;
use crate::{rng::rand_slice, traits::SealedKey, CryptoError as Error};
use anyhow::anyhow;
use log::*;
use secp256k1::curve::Scalar;
use secp256k1::{
    util::{COMPRESSED_PUBLIC_KEY_SIZE, SECRET_KEY_SIZE},
    Message, PublicKey, RecoveryId, SecretKey, Signature,
};
use sgx_types::sgx_report_data_t;
use std::format;
use std::io::{Read, Write};
use std::sgxfs::SgxFile;
use std::vec;
use std::vec::Vec;
use store::{CommitSigner, CommitVerifier, StoreError};

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

    pub fn sign_hash(&self, bz: &[u8; 32]) -> Result<Vec<u8>, Error> {
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

impl EnclavePublicKey {
    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let pk = PublicKey::parse_slice(bz, None).map_err(Error::Secp256k1Error)?;
        Ok(Self(pk))
    }

    pub fn as_bytes(&self) -> [u8; COMPRESSED_PUBLIC_KEY_SIZE] {
        self.0.serialize_compressed()
    }

    pub fn get_address(&self) -> [u8; 20] {
        let pubkey = &self.as_bytes()[1..];
        let account_id = &pubkey.keccak256()[12..];
        let mut res: [u8; 20] = Default::default();
        res.copy_from_slice(account_id);
        res
    }

    pub fn as_report_data(&self) -> sgx_report_data_t {
        let mut report_data = sgx_report_data_t::default();
        report_data.d[..20].copy_from_slice(&self.get_address()[..]);
        report_data
    }

    pub fn verify(&self, msg: &[u8; 32], signature: &[u8]) -> Result<(), Error> {
        assert!(signature.len() == 65);

        let mut s = Scalar::default();
        let _ = s.set_b32(msg);

        let sig = Signature::parse_slice(&signature[..64]).map_err(Error::Secp256k1Error)?;
        let rid = RecoveryId::parse(signature[64]).map_err(Error::Secp256k1Error)?;
        let signer = secp256k1::recover(&Message(s), &sig, &rid).map_err(Error::Secp256k1Error)?;
        if self.0.eq(&signer) {
            Ok(())
        } else {
            Err(Error::VerificationError(format!(
                "unexpected signer: {:?}",
                signer
            )))
        }
    }
}

impl CommitSigner for EnclaveKey {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, StoreError> {
        self.sign(msg)
            .map_err(|e| StoreError::OtherError(anyhow!(e)))
    }

    fn sign_hash(&self, msg: &[u8; 32]) -> Result<Vec<u8>, StoreError> {
        self.sign_hash(msg)
            .map_err(|e| StoreError::OtherError(anyhow!(e)))
    }

    fn use_verifier(&self, f: &mut dyn FnMut(&dyn CommitVerifier)) {
        f(&self.get_pubkey());
    }
}

impl CommitVerifier for EnclavePublicKey {
    fn get_pubkey(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn get_address(&self) -> Vec<u8> {
        self.get_address().to_vec()
    }

    fn verify(&self, msg: &[u8; 32], signature: &[u8]) -> Result<(), StoreError> {
        self.verify(msg, signature)
            .map_err(|e| StoreError::OtherError(anyhow!(e)))?;
        Ok(())
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
    let mut file = SgxFile::create(filepath).map_err(|_err| {
        error!("error creating file {}: {:?}", filepath, _err);
        Error::FailedSeal
    })?;

    file.write_all(data).map_err(|_err| {
        error!("error writing to path {}: {:?}", filepath, _err);
        Error::FailedSeal
    })
}

fn open(filepath: &str) -> Result<SecretKey, Error> {
    let mut file = SgxFile::open(filepath).map_err(|_err| Error::FailedUnseal)?;

    let mut buf = [0u8; SECRET_KEY_SIZE];
    let n = file
        .read(buf.as_mut())
        .map_err(|_err| Error::FailedUnseal)?;

    if n < SECRET_KEY_SIZE {
        error!(
            "[Enclave] Dramatic read from {} ended prematurely (n = {} < SECRET_KEY_SIZE = {})",
            filepath, n, SECRET_KEY_SIZE
        );
        return Err(Error::FailedUnseal);
    }
    Ok(SecretKey::parse(&buf).unwrap())
}

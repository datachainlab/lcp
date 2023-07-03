use crate::prelude::*;
use crate::EnclavePublicKey;
use crate::Error;
use crate::SealedEnclaveKey;
use tiny_keccak::Keccak;

pub trait Verifier {
    fn verify(&self, msg: &[u8], signature: &[u8]) -> Result<(), Error>;
}

pub trait Signer {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error>;
    fn pubkey(&self) -> Result<EnclavePublicKey, Error>;
}

pub trait SealingKey
where
    Self: core::marker::Sized,
{
    fn seal(&self) -> Result<SealedEnclaveKey, Error>;
    fn unseal(sek: &SealedEnclaveKey) -> Result<Self, Error>;
}

pub trait Keccak256 {
    fn keccak256(&self) -> [u8; 32];
}

impl Keccak256 for [u8] {
    fn keccak256(&self) -> [u8; 32] {
        let mut keccak = Keccak::new_keccak256();
        let mut result = [0u8; 32];
        keccak.update(self);
        keccak.finalize(result.as_mut());
        result
    }
}

impl Keccak256 for Vec<u8> {
    fn keccak256(&self) -> [u8; 32] {
        self.as_slice().keccak256()
    }
}

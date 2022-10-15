use crate::prelude::*;
use crate::Error;
use tiny_keccak::Keccak;

pub trait Verifier {
    fn get_pubkey(&self) -> Vec<u8>;
    fn get_address(&self) -> Vec<u8>;
    fn verify(&self, msg: &[u8], signature: &[u8]) -> Result<(), Error>;
}

pub trait Signer {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error>;
    fn use_verifier(&self, f: &mut dyn FnMut(&dyn Verifier));
}

pub trait SealedKey
where
    Self: core::marker::Sized,
{
    fn seal(&self, filepath: &str) -> Result<(), Error>;
    fn unseal(filepath: &str) -> Result<Self, Error>;
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

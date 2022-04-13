// TODO: Refactoring enclave-crypto to be able to use it under std env and remove duplicated definitions in this crate

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use secp256k1::curve::Scalar;
use secp256k1::{Message, PublicKey, RecoveryId, SecretKey, Signature};
use std::vec::Vec;
use tiny_keccak::Keccak;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Address([u8; 20]);

impl From<&[u8]> for Address {
    fn from(v: &[u8]) -> Self {
        assert!(v.len() == 20);
        let mut addr = Address::default();
        addr.0.copy_from_slice(v);
        addr
    }
}

pub fn verify_signature(sign_bytes: &[u8], signature: &[u8]) -> Result<Address, ()> {
    assert!(signature.len() == 65);

    let sign_hash = keccak256(sign_bytes);
    let mut s = Scalar::default();
    let _ = s.set_b32(&sign_hash);

    let sig = Signature::parse_slice(&signature[..64]).unwrap();
    let rid = RecoveryId::parse(signature[64]).unwrap();
    let signer = secp256k1::recover(&Message(s), &sig, &rid).unwrap();
    Ok(pubkey_to_address(&signer))
}

fn pubkey_to_address(pubkey: &PublicKey) -> Address {
    let pubkey = &pubkey.serialize_compressed()[1..];
    let account_id = &keccak256(pubkey)[12..];
    let mut res: Address = Default::default();
    res.0.copy_from_slice(account_id);
    res
}

fn keccak256(bz: &[u8]) -> [u8; 32] {
    let mut keccak = Keccak::new_keccak256();
    let mut result = [0u8; 32];
    keccak.update(bz);
    keccak.finalize(result.as_mut());
    result
}

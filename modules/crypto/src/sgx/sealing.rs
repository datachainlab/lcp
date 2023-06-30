use crate::prelude::*;
use crate::traits::SealingKey;
use crate::EnclaveKey;
use crate::Error;
use crate::Signer;
use crate::Verifier;
use libsecp256k1::{util::SECRET_KEY_SIZE, SecretKey};
use sgx_tseal::SgxSealedData;
use sgx_types::{marker::ContiguousMemory, sgx_sealed_data_t};

pub const SEALED_DATA_32_SIZE: u32 = calc_raw_sealed_data_size(0, 32);
pub const SEALED_DATA_32_USIZE: usize = safe_u32_to_usize(SEALED_DATA_32_SIZE);

#[derive(Clone, Copy)]
struct UnsealedEnclaveKey([u8; SECRET_KEY_SIZE]);

unsafe impl ContiguousMemory for UnsealedEnclaveKey {}

impl SealingKey for EnclaveKey {
    fn seal(&self) -> Result<Vec<u8>, Error> {
        seal_enclave_key(UnsealedEnclaveKey(self.get_privkey()))
    }

    fn unseal(bz: Vec<u8>) -> Result<Self, Error> {
        if bz.len() != SEALED_DATA_32_USIZE {
            return Err(Error::failed_unseal("".to_owned()));
        }
        let mut data = [0; SEALED_DATA_32_USIZE];
        data.copy_from_slice(bz.as_slice());
        let unsealed = unseal_enclave_key(data)?;
        let secret_key = SecretKey::parse(&unsealed.0)?;
        Ok(Self { secret_key })
    }
}

fn seal_enclave_key(data: UnsealedEnclaveKey) -> Result<Vec<u8>, Error> {
    let sealed_data = SgxSealedData::<UnsealedEnclaveKey>::seal_data(Default::default(), &data)?;
    let mut buf = Vec::with_capacity(SEALED_DATA_32_USIZE);
    let _ = unsafe {
        sealed_data.to_raw_sealed_data_t(
            buf.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    };
    Ok(buf)
}

fn unseal_enclave_key(mut bz: [u8; SEALED_DATA_32_USIZE]) -> Result<UnsealedEnclaveKey, Error> {
    let sealed = unsafe {
        SgxSealedData::<UnsealedEnclaveKey>::from_raw_sealed_data_t(
            bz.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    }
    .ok_or_else(|| Error::failed_unseal("".to_owned()))?;
    Ok(*sealed.unseal_data()?.get_decrypt_txt())
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

const fn safe_u32_to_usize(v: u32) -> usize {
    assert!(usize::BITS >= 32);
    v as usize
}

pub struct SealedEnclaveKey([u8; SEALED_DATA_32_USIZE]);

impl SealedEnclaveKey {
    pub fn new(sealed_key: [u8; SEALED_DATA_32_USIZE]) -> Self {
        Self(sealed_key)
    }

    pub fn new_from_bytes(bz: &[u8]) -> Result<Self, Error> {
        if bz.len() != SEALED_DATA_32_USIZE {
            return Err(Error::failed_unseal("".to_owned()));
        }
        let mut data = [0; SEALED_DATA_32_USIZE];
        data.copy_from_slice(bz);
        Ok(Self::new(data))
    }
}

impl Signer for SealedEnclaveKey {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        EnclaveKey::unseal(self.0.to_vec())?.sign(msg)
    }

    // TODO remove this method
    fn use_verifier(&self, f: &mut dyn FnMut(&dyn Verifier)) {
        EnclaveKey::unseal(self.0.to_vec()).unwrap().use_verifier(f)
    }
}

pub fn validate_sealed_enclave_key(bz: &[u8]) -> Result<(), Error> {
    if bz.len() != SEALED_DATA_32_USIZE {
        Err(Error::invalid_sealed_enclave_key(format!(
            "unexpected length: expected={} actual={}",
            SEALED_DATA_32_USIZE,
            bz.len()
        )))
    } else {
        Ok(())
    }
}

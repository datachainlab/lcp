use crate::key::{SealedEnclaveKey, SEALED_DATA_32_SIZE, SEALED_DATA_32_USIZE};
use crate::traits::SealingKey;
use crate::EnclaveKey;
use crate::Error;
use crate::Signer;
use crate::{prelude::*, EnclavePublicKey};
use libsecp256k1::{util::SECRET_KEY_SIZE, SecretKey};
use sgx_tseal::SgxSealedData;
use sgx_types::{marker::ContiguousMemory, sgx_sealed_data_t};

#[derive(Clone, Copy)]
struct UnsealedEnclaveKey([u8; SECRET_KEY_SIZE]);

unsafe impl ContiguousMemory for UnsealedEnclaveKey {}

impl SealingKey for EnclaveKey {
    fn seal(&self) -> Result<SealedEnclaveKey, Error> {
        seal_enclave_key(UnsealedEnclaveKey(self.get_privkey()))
    }

    fn unseal(sek: &SealedEnclaveKey) -> Result<Self, Error> {
        let unsealed = unseal_enclave_key(&sek)?;
        let secret_key = SecretKey::parse(&unsealed.0)?;
        Ok(Self { secret_key })
    }
}

fn seal_enclave_key(data: UnsealedEnclaveKey) -> Result<SealedEnclaveKey, Error> {
    let sealed_data = SgxSealedData::<UnsealedEnclaveKey>::seal_data(Default::default(), &data)?;
    let mut sek = SealedEnclaveKey([0; SEALED_DATA_32_USIZE]);
    let _ = unsafe {
        sealed_data.to_raw_sealed_data_t(
            sek.0.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    };
    Ok(sek)
}

fn unseal_enclave_key(sek: &SealedEnclaveKey) -> Result<UnsealedEnclaveKey, Error> {
    let mut sek = sek.clone();
    let sealed = unsafe {
        SgxSealedData::<UnsealedEnclaveKey>::from_raw_sealed_data_t(
            sek.0.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    }
    .ok_or_else(|| Error::failed_unseal("failed to unseal data".to_owned()))?;
    Ok(*sealed.unseal_data()?.get_decrypt_txt())
}

impl Signer for SealedEnclaveKey {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        EnclaveKey::unseal(self)?.sign(msg)
    }
    fn pubkey(&self) -> Result<EnclavePublicKey, Error> {
        Ok(EnclaveKey::unseal(self)?.get_pubkey())
    }
}

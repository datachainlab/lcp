use crate::key::{SealedEnclaveKey, SEALED_DATA_32_SIZE, SEALED_DATA_32_USIZE};
use crate::traits::SealingKey;
use crate::EnclaveKey;
use crate::Error;
use crate::Signer;
use crate::{prelude::*, EnclavePublicKey};
use libsecp256k1::{util::SECRET_KEY_SIZE, SecretKey};
use sgx_tseal::SgxSealedData;
use sgx_types::{marker::ContiguousMemory, sgx_sealed_data_t};
use sgx_types::{sgx_attributes_t, SGX_KEYPOLICY_MRENCLAVE, TSEAL_DEFAULT_MISCMASK};

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
    let attribute_mask = sgx_attributes_t {
        flags: 0xffff_ffff_ffff_fff3,
        xfrm: 0,
    };
    let sealed_data = SgxSealedData::<UnsealedEnclaveKey>::seal_data_ex(
        SGX_KEYPOLICY_MRENCLAVE,
        attribute_mask,
        TSEAL_DEFAULT_MISCMASK,
        Default::default(),
        &data,
    )
    .map_err(|e| Error::sgx_error(e, "failed to seal enclave key".to_string()))?;
    let mut sek = SealedEnclaveKey([0; SEALED_DATA_32_USIZE]);
    match unsafe {
        sealed_data.to_raw_sealed_data_t(
            sek.0.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    } {
        Some(_) => Ok(sek),
        None => Err(Error::failed_seal(
            "failed to convert to raw sealed data".to_owned(),
        )),
    }
}

fn unseal_enclave_key(sek: &SealedEnclaveKey) -> Result<UnsealedEnclaveKey, Error> {
    let mut sek = sek.clone();
    let sealed = unsafe {
        SgxSealedData::<UnsealedEnclaveKey>::from_raw_sealed_data_t(
            sek.0.as_mut_ptr() as *mut sgx_sealed_data_t,
            SEALED_DATA_32_SIZE,
        )
    }
    .ok_or_else(|| Error::failed_unseal("failed to convert from raw sealed data".to_owned()))?;
    Ok(*sealed
        .unseal_data()
        .map_err(|e| Error::sgx_error(e, "failed to unseal enclave key".to_string()))?
        .get_decrypt_txt())
}

impl Signer for SealedEnclaveKey {
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        EnclaveKey::unseal(self)?.sign(msg)
    }
    fn pubkey(&self) -> Result<EnclavePublicKey, Error> {
        Ok(EnclaveKey::unseal(self)?.get_pubkey())
    }
}

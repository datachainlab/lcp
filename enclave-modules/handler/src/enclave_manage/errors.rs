use derive_more::Display;
use enclave_crypto::CryptoError;
use sgx_types::sgx_status_t;

#[derive(thiserror::Error, Debug, Display)]
pub enum EnclaveManageError {
    SGXError(sgx_status_t),
    CryptoError(CryptoError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use sgx_types::sgx_status_t;

#[derive(thiserror::Error, Debug)]
pub enum EnclaveManageError {
    #[error("SGXError: {0}")]
    SGXError(sgx_status_t),
    #[error("CryptoError")]
    CryptoError(#[from] crypto::CryptoError),
    #[error("AttestationReportError")]
    AttestationReportError(#[from] attestation_report::AttestationReportError),
    #[error("RemoteAttestationError")]
    RemoteAttestationError(#[from] enclave_remote_attestation::errors::RemoteAttestationError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

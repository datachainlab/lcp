#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use attestation_report::AttestationReportError;
use crypto::CryptoError;
use derive_more::Display;
use sgx_types::sgx_status_t;

#[derive(thiserror::Error, Debug, Display)]
pub enum EnclaveManageError {
    SGXError(sgx_status_t),
    CryptoError(CryptoError),
    AttestationReportError(AttestationReportError),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

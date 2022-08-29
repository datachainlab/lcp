#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use log_sgx as log;
    pub use secp256k1_sgx as secp256k1;
    pub use thiserror_sgx as thiserror;
}

pub use crate::secp256k1::{verify_signature, Address, EnclaveKey, EnclavePublicKey};
pub use errors::CryptoError;
pub use key_manager::KeyManager;
pub use traits::{SealedKey, Signer, Verifier};

mod errors;
mod key_manager;
mod secp256k1;
#[cfg(feature = "sgx")]
pub mod sgx;
mod traits;

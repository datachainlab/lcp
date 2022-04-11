#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;
extern crate sgx_types;

pub use crate::secp256k1::{EnclaveKey, EnclavePublicKey};
pub use errors::CryptoError;
pub use key_manager::KeyManager;
pub use rand::fill_bytes as rand_fill_bytes;
pub use traits::SealedKey;

pub mod consts;
mod errors;
mod key_manager;
mod rand;
mod rng;
mod secp256k1;
mod traits;

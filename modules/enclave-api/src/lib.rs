pub use api::{EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI};
pub use enclave::Enclave;
use errors::{Error, Result};

mod api;
mod enclave;
mod errors;
mod ffi;
mod memory;
#[cfg(feature = "rocksdb")]
mod rocksdb;

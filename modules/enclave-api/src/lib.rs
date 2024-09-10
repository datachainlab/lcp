pub use api::{EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI};
pub use enclave::{Enclave, EnclaveInfo};
pub use errors::Error;
use errors::Result;

mod api;
mod enclave;
mod errors;
mod ffi;
mod memory;
#[cfg(feature = "rocksdb")]
mod rocksdb;

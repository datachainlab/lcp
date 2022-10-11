pub use api::{EnclavePrimitiveAPI, EnclaveProtoAPI};
pub use enclave::Enclave;
use errors::{Error, Result};

mod api;
mod enclave;
mod errors;
mod ffi;

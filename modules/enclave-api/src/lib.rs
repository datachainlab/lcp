pub use api::{EnclavePrimitiveAPI, EnclaveProtoAPI};
pub use enclave::Enclave;
use errors::{EnclaveAPIError, Result};

mod api;
mod enclave;
mod errors;
mod ffi;

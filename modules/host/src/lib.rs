pub use enclave::{create_enclave, sgx_get_metadata};
pub use ocalls::{get_environment, ocall_execute_command, set_environment, SetEnvironmentError};

mod enclave;
mod ocalls;

pub use enclave::load_enclave;
pub use ocalls::{get_environment, ocall_execute_command, set_environment, SetEnvironmentError};

mod enclave;
mod ocalls;

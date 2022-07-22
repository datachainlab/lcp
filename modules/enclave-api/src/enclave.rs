use sgx_urts::SgxEnclave;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct Enclave {
    pub(crate) home: String,
    pub(crate) sgx_enclave: SgxEnclave,
}

impl Enclave {
    pub fn new(sgx_enclave: SgxEnclave, home: String) -> Self {
        Enclave { home, sgx_enclave }
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }

    pub fn current_timestamp() -> u128 {
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        current_timestamp.as_nanos()
    }
}

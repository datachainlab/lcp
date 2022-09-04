use lcp_types::Time;
use sgx_urts::SgxEnclave;

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

    pub fn current_timestamp() -> Time {
        Time::now()
    }
}

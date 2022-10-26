use crate::EnclaveProtoAPI;
use host_environment::Environment;
use lcp_types::Time;
use sgx_urts::SgxEnclave;
use std::{marker::PhantomData, ops::DerefMut};
use store::host::{HostCommitStore, HostStore, HostStoreAccessor};
use store::transaction::CommitStore;

pub struct Enclave<'e, S: CommitStore> {
    pub(crate) sgx_enclave: SgxEnclave,
    pub(crate) env: &'e Environment,
    _marker: PhantomData<S>,
}

impl<'e, S: CommitStore> Enclave<'e, S>
where
    Self: EnclaveProtoAPI<S>,
{
    pub fn new(sgx_enclave: SgxEnclave, env: &'e Environment) -> Self {
        Enclave {
            sgx_enclave,
            env,
            _marker: PhantomData::default(),
        }
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }
}

pub trait EnclaveInfo {
    fn get_home(&self) -> String;
    fn get_eid(&self) -> sgx_types::sgx_enclave_id_t;
    fn current_timestamp(&self) -> Time;
}

impl<'e, S: CommitStore> EnclaveInfo for Enclave<'e, S> {
    fn get_home(&self) -> String {
        self.env.home.to_str().unwrap().to_string()
    }

    fn get_eid(&self) -> sgx_types::sgx_enclave_id_t {
        self.sgx_enclave.geteid()
    }

    fn current_timestamp(&self) -> Time {
        Time::now()
    }
}

impl<'e, S> HostStoreAccessor<S> for Enclave<'e, S>
where
    S: CommitStore,
    HostStore: HostCommitStore<S>,
{
    fn use_mut_store<T>(&self, f: impl FnOnce(&mut S) -> T) -> T {
        let mut store = self.env.get_mut_store();
        store.deref_mut().apply(f)
    }
}

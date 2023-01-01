use crate::{
    enclave::HostStoreTxManager, Enclave, EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI,
};
use store::memory::MemStore;

impl HostStoreTxManager<MemStore> for Enclave<MemStore> {}
impl EnclavePrimitiveAPI<MemStore> for Enclave<MemStore> {}
impl EnclaveCommandAPI<MemStore> for Enclave<MemStore> {}
impl EnclaveProtoAPI<MemStore> for Enclave<MemStore> {}

use crate::{Enclave, EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI};
use store::memory::MemStore;
use store::transaction::TxManager;

impl<'e> TxManager<MemStore> for Enclave<'e, MemStore> {}
impl<'e> EnclavePrimitiveAPI<MemStore> for Enclave<'e, MemStore> {}
impl<'e> EnclaveCommandAPI<MemStore> for Enclave<'e, MemStore> {}
impl<'e> EnclaveProtoAPI<MemStore> for Enclave<'e, MemStore> {}

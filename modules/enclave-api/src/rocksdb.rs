use crate::{
    enclave::HostStoreTxManager, Enclave, EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI,
};
use store::rocksdb::RocksDBStore;

impl<'e> HostStoreTxManager<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclavePrimitiveAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclaveCommandAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclaveProtoAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}

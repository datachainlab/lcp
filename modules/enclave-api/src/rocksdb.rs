use crate::{
    enclave::HostStoreTxManager, Enclave, EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI,
};
use store::rocksdb::RocksDBStore;

impl HostStoreTxManager<RocksDBStore> for Enclave<RocksDBStore> {}
impl EnclavePrimitiveAPI<RocksDBStore> for Enclave<RocksDBStore> {}
impl EnclaveCommandAPI<RocksDBStore> for Enclave<RocksDBStore> {}
impl EnclaveProtoAPI<RocksDBStore> for Enclave<RocksDBStore> {}

use crate::{Enclave, EnclaveCommandAPI, EnclavePrimitiveAPI, EnclaveProtoAPI};
use store::rocksdb::RocksDBStore;
use store::transaction::TxManager;

impl<'e> TxManager<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclavePrimitiveAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclaveCommandAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}
impl<'e> EnclaveProtoAPI<RocksDBStore> for Enclave<'e, RocksDBStore> {}

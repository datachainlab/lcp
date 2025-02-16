use crate::errors::Result;
use keymanager::EnclaveKeyManager;
use sgx_types::{metadata::metadata_t, sgx_enclave_id_t, SgxResult};
use sgx_urts::SgxEnclave;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{marker::PhantomData, ops::DerefMut};
use store::host::{HostStore, IntoCommitStore};
use store::transaction::{CommitStore, CreatedTx, UpdateKey};

/// `Enclave` keeps an enclave id and reference to the host environement
pub struct Enclave<S: CommitStore> {
    pub(crate) path: PathBuf,
    pub(crate) key_manager: EnclaveKeyManager,
    pub(crate) store: Arc<RwLock<HostStore>>,
    pub(crate) sgx_enclave: SgxEnclave,
    _marker: PhantomData<S>,
}

impl<S: CommitStore> Enclave<S> {
    pub fn new(
        path: impl Into<PathBuf>,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
        sgx_enclave: SgxEnclave,
    ) -> Self {
        Enclave {
            path: path.into(),
            key_manager,
            store,
            sgx_enclave,
            _marker: PhantomData,
        }
    }

    pub fn create(
        path: impl Into<PathBuf>,
        debug: bool,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
    ) -> SgxResult<Self> {
        let path = path.into();
        let enclave = host::create_enclave(path.clone(), debug)?;
        Ok(Self::new(path, key_manager, store, enclave))
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }
}

/// `EnclaveInfo` is an accessor to enclave information
pub trait EnclaveInfo: Sync + Send {
    /// `get_eid` returns the enclave id
    fn get_eid(&self) -> sgx_enclave_id_t;
    /// `metadata` returns the metadata of the enclave
    fn metadata(&self) -> SgxResult<metadata_t>;
    /// `is_debug` returns true if the enclave is in debug mode
    fn is_debug(&self) -> bool;
    /// `get_key_manager` returns a key manager for Enclave Keys
    fn get_key_manager(&self) -> &EnclaveKeyManager;
}

impl<S: CommitStore> EnclaveInfo for Enclave<S> {
    /// `get_eid` returns the enclave id
    fn get_eid(&self) -> sgx_enclave_id_t {
        self.sgx_enclave.geteid()
    }
    /// `metadata` returns the metadata of the enclave
    fn metadata(&self) -> SgxResult<metadata_t> {
        host::sgx_get_metadata(&self.path)
    }
    /// `is_debug` returns true if the enclave is in debug mode
    fn is_debug(&self) -> bool {
        self.sgx_enclave.is_debug()
    }
    /// `get_keymanager` returns a key manager for Enclave Keys
    fn get_key_manager(&self) -> &EnclaveKeyManager {
        &self.key_manager
    }
}

/// `HostStoreTxManager` is a transaction manager for the host store
pub trait HostStoreTxManager<S: CommitStore>: CommitStoreAccessor<S> {
    /// `begin_tx` creates a transaction and begin it
    fn begin_tx(&self, update_key: Option<UpdateKey>) -> Result<<S::Tx as CreatedTx>::PreparedTx> {
        let tx = self.use_mut_store(|store| store.create_transaction(update_key))?;
        let tx = tx.prepare()?;
        self.use_mut_store(|store| store.begin(&tx))?;
        Ok(tx)
    }

    /// `commit_tx` commits the changes in the transaction
    fn commit_tx(&self, tx: <S::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.use_mut_store(|store| store.commit(tx))?;
        Ok(())
    }

    /// `rollback_tx` rollbacks the changes in the transaction
    fn rollback_tx(&self, tx: <S::Tx as CreatedTx>::PreparedTx) {
        self.use_mut_store(|store| store.rollback(tx));
    }
}

/// `CommitStoreAccessor` is an accessor to the host store
pub trait CommitStoreAccessor<S: CommitStore> {
    fn use_mut_store<T>(&self, f: impl FnOnce(&mut S) -> T) -> T;
}

impl<S> CommitStoreAccessor<S> for Enclave<S>
where
    S: CommitStore,
    HostStore: IntoCommitStore<S>,
{
    fn use_mut_store<T>(&self, f: impl FnOnce(&mut S) -> T) -> T {
        let mut store = self.store.write().unwrap();
        store.deref_mut().apply(f)
    }
}

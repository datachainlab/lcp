use crate::errors::Result;
use lcp_types::Time;
use sgx_types::metadata::metadata_t;
use sgx_types::SgxResult;
use sgx_urts::SgxEnclave;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{marker::PhantomData, ops::DerefMut};
use store::host::{HostStore, IntoCommitStore};
use store::transaction::{CommitStore, CreatedTx, UpdateKey};

/// `Enclave` keeps an enclave id and reference to the host environement
pub struct Enclave<S> {
    pub(crate) path: PathBuf,
    pub(crate) home_path: PathBuf,
    pub(crate) store: Arc<RwLock<HostStore>>,
    pub(crate) sgx_enclave: SgxEnclave,
    _marker: PhantomData<S>,
}

impl<S> Enclave<S> {
    pub fn new(
        path: impl Into<PathBuf>,
        home_path: impl Into<PathBuf>,
        store: Arc<RwLock<HostStore>>,
        sgx_enclave: SgxEnclave,
    ) -> Self {
        Enclave {
            path: path.into(),
            home_path: home_path.into(),
            store,
            sgx_enclave,
            _marker: PhantomData::default(),
        }
    }

    pub fn create(
        path: impl Into<PathBuf>,
        home_path: impl Into<PathBuf>,
        store: Arc<RwLock<HostStore>>,
    ) -> SgxResult<Self> {
        let path = path.into();
        let enclave = host::create_enclave(path.clone())?;
        Ok(Self::new(path, home_path, store, enclave))
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }
}

/// `EnclaveInfo` is an accessor to enclave information
pub trait EnclaveInfo {
    fn get_home(&self) -> String;
    fn get_eid(&self) -> sgx_types::sgx_enclave_id_t;
    fn current_timestamp(&self) -> Time;
    fn metadata(&self) -> SgxResult<metadata_t>;
}

impl<S> EnclaveInfo for Enclave<S> {
    fn get_home(&self) -> String {
        self.home_path.to_str().unwrap().to_string()
    }

    fn get_eid(&self) -> sgx_types::sgx_enclave_id_t {
        self.sgx_enclave.geteid()
    }

    fn current_timestamp(&self) -> Time {
        Time::now()
    }

    fn metadata(&self) -> SgxResult<metadata_t> {
        host::sgx_get_metadata(&self.path)
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

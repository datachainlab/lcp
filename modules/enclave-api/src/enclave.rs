use crate::errors::{Error, Result};
use keymanager::EnclaveKeyManager;
use lcp_types::{store_key, Any, EnclaveMetadata, Height};
use sgx_types::{sgx_enclave_id_t, SgxResult};
use sgx_urts::SgxEnclave;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{marker::PhantomData, ops::DerefMut};
use store::host::{HostStore, IntoCommitStore};
use store::transaction::{CommitStore, CreatedTx, Tx, TxAccessor, UpdateKey};
use store::WriteSet;

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

/// `EnclaveInfo` is an accessor to enclave information.
///
/// Concurrency over `ecall_execute_command` is the caller's responsibility:
/// the LCP service routes all ECALLs through `service::EcallPool`, which pins
/// host threads that issue ECALLs to a fixed set so that cumulative TCS
/// bindings under `TCSPolicy=BIND` cannot exceed the pool size.
pub trait EnclaveInfo: Sync + Send {
    /// `get_eid` returns the enclave id
    fn get_eid(&self) -> sgx_enclave_id_t;
    /// `metadata` returns the metadata of the enclave
    fn metadata(&self) -> SgxResult<EnclaveMetadata>;
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
    fn metadata(&self) -> SgxResult<EnclaveMetadata> {
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

    /// `begin_speculative_tx` creates a transaction whose writes remain isolated
    /// from the canonical store until an explicit stitch/commit phase exists above this layer.
    fn begin_speculative_tx(&self) -> Result<<S::Tx as CreatedTx>::PreparedTx> {
        let tx = self.use_mut_store(|store| store.create_speculative_transaction())?;
        let tx = tx.prepare()?;
        self.use_mut_store(|store| store.begin(&tx))?;
        Ok(tx)
    }

    /// `commit_tx` commits the changes in the transaction
    fn commit_tx(&self, tx: <S::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.use_mut_store(|store| store.commit(tx))?;
        Ok(())
    }

    /// `apply_write_set` applies a speculative write set to the canonical store under a
    /// serialized update transaction keyed by `update_key`.
    fn apply_write_set(&self, update_key: UpdateKey, write_set: WriteSet) -> Result<()>
    where
        S: TxAccessor,
    {
        let tx = self.begin_tx(Some(update_key))?;
        let tx_id = tx.get_id();
        if let Err(e) = self.apply_write_set_in_tx(tx_id, write_set) {
            self.rollback_tx(tx);
            return Err(e);
        }
        self.commit_tx(tx)
    }

    /// `apply_write_set_with_expected_base` applies a speculative write set only if the
    /// store already contains the explicit base state that seeded the batch at
    /// `prev_height`.
    ///
    /// The check and apply run under the same serialized update transaction keyed by
    /// `update_key`, so the accepted base cannot change between verification and commit.
    /// The explicit base client state must match the latest canonical
    /// client_state. This prevents an old, historically valid base state from
    /// overwriting a newer latest-only client_state. The caller-supplied
    /// `prev_state_id` (observed in-enclave by the first speculative unit)
    /// must also match the height-indexed state ID previously stored by a
    /// successful create/serial/speculative update.
    fn apply_write_set_with_expected_base(
        &self,
        update_key: UpdateKey,
        prev_height: Height,
        client_state: &Any,
        consensus_state: &Any,
        prev_state_id: Option<&[u8]>,
        write_set: WriteSet,
    ) -> Result<()>
    where
        S: TxAccessor,
    {
        let tx = self.begin_tx(Some(update_key.clone()))?;
        let tx_id = tx.get_id();
        if let Err(e) = self.verify_expected_base_state_in_tx(
            tx_id,
            &update_key,
            &prev_height,
            client_state,
            consensus_state,
            prev_state_id,
        ) {
            self.rollback_tx(tx);
            return Err(e);
        }
        if let Err(e) = self.apply_write_set_in_tx(tx_id, write_set) {
            self.rollback_tx(tx);
            return Err(e);
        }
        self.commit_tx(tx)
    }

    fn apply_write_set_in_tx(&self, tx_id: store::TxId, write_set: WriteSet) -> Result<()>
    where
        S: TxAccessor,
    {
        for (key, value) in write_set {
            self.use_mut_store(|store| match value {
                Some(value) => store.tx_set(tx_id, key, value),
                None => store.tx_remove(tx_id, &key),
            })?;
        }
        Ok(())
    }

    fn verify_expected_base_state_in_tx(
        &self,
        tx_id: store::TxId,
        client_id: &str,
        prev_height: &Height,
        _client_state: &Any,
        _consensus_state: &Any,
        prev_state_id: Option<&[u8]>,
    ) -> Result<()>
    where
        S: TxAccessor,
    {
        // The supplied Anys are intentionally not byte-compared. The
        // observed `prev_state_id` from the in-enclave light client is
        // `gen_state_id(canonicalize(client_state), canonicalize(consensus_state))`,
        // and `stored_state_id` was written by the same canonicalization
        // at commit time. Comparing state_ids therefore checks canonical
        // equivalence and absorbs encoding-only differences in the raw
        // Any bytes; value-level divergence at the same height still
        // flows through canonicalize() into state_id and is rejected.
        // The supplied bytes are still seeded into the speculative
        // transaction via `compute_seed_write_set` so the in-enclave
        // light client observes exactly the supplied state.
        let prev_state_id = prev_state_id.ok_or_else(|| {
            Error::invalid_argument(format!(
                "speculative update_client must provide prev_state_id: client_id={} height={}-{}",
                client_id,
                prev_height.revision_number(),
                prev_height.revision_height()
            ))
        })?;
        let state_id_key = store_key::state_id_bytes(client_id, prev_height);
        let stored_state_id = self.use_mut_store(|store| store.tx_get(tx_id, &state_id_key))?;
        let Some(stored_state_id) = stored_state_id else {
            return Err(Error::invalid_argument(format!(
                "stored speculative base state_id missing: client_id={} height={}-{}",
                client_id,
                prev_height.revision_number(),
                prev_height.revision_height()
            )));
        };
        if stored_state_id.as_slice() != prev_state_id {
            return Err(Error::invalid_argument(format!(
                "stored speculative base state_id mismatch: client_id={} height={}-{}",
                client_id,
                prev_height.revision_number(),
                prev_height.revision_height()
            )));
        }
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

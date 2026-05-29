use crate::errors::{Error, Result};
use keymanager::EnclaveKeyManager;
use lcp_types::{store_key, Any, EnclaveMetadata, Height};
use sgx_types::{sgx_enclave_id_t, SgxResult};
use sgx_urts::SgxEnclave;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
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
    pub(crate) ecall_gate: Arc<ECallGate>,
    _marker: PhantomData<S>,
}

#[derive(Debug)]
pub(crate) struct ECallGate {
    state: Mutex<ECallGateState>,
    ready: Condvar,
}

#[derive(Debug)]
struct ECallGateState {
    available: usize,
}

struct ECallPermitGuard<'a> {
    gate: &'a ECallGate,
}

impl<S: CommitStore> Enclave<S> {
    pub const DEFAULT_ECALL_CONCURRENCY: usize = 4;

    pub fn new(
        path: impl Into<PathBuf>,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
        sgx_enclave: SgxEnclave,
    ) -> Self {
        Self::new_with_ecall_concurrency(
            path,
            key_manager,
            store,
            sgx_enclave,
            Self::DEFAULT_ECALL_CONCURRENCY,
        )
    }

    pub fn new_with_ecall_concurrency(
        path: impl Into<PathBuf>,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
        sgx_enclave: SgxEnclave,
        ecall_concurrency: usize,
    ) -> Self {
        Enclave {
            path: path.into(),
            key_manager,
            store,
            sgx_enclave,
            ecall_gate: Arc::new(ECallGate::new(ecall_concurrency)),
            _marker: PhantomData,
        }
    }

    pub fn create(
        path: impl Into<PathBuf>,
        debug: bool,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
    ) -> SgxResult<Self> {
        Self::create_with_ecall_concurrency(
            path,
            debug,
            key_manager,
            store,
            Self::DEFAULT_ECALL_CONCURRENCY,
        )
    }

    pub fn create_with_ecall_concurrency(
        path: impl Into<PathBuf>,
        debug: bool,
        key_manager: EnclaveKeyManager,
        store: Arc<RwLock<HostStore>>,
        ecall_concurrency: usize,
    ) -> SgxResult<Self> {
        let path = path.into();
        let enclave = host::create_enclave(path.clone(), debug)?;
        Ok(Self::new_with_ecall_concurrency(
            path,
            key_manager,
            store,
            enclave,
            ecall_concurrency,
        ))
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }
}

impl ECallGate {
    fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(ECallGateState {
                available: permits.max(1),
            }),
            ready: Condvar::new(),
        }
    }

    fn with_permit<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let _permit = self.acquire();
        f()
    }

    fn acquire(&self) -> ECallPermitGuard<'_> {
        let mut state = self.state.lock().unwrap();
        while state.available == 0 {
            state = self.ready.wait(state).unwrap();
        }
        state.available -= 1;
        ECallPermitGuard { gate: self }
    }
}

impl Drop for ECallPermitGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.state.lock().unwrap();
        state.available += 1;
        self.gate.ready.notify_one();
    }
}

/// `EnclaveInfo` is an accessor to enclave information
pub trait EnclaveInfo: Sync + Send {
    /// `get_eid` returns the enclave id
    fn get_eid(&self) -> sgx_enclave_id_t;
    /// `metadata` returns the metadata of the enclave
    fn metadata(&self) -> SgxResult<EnclaveMetadata>;
    /// `is_debug` returns true if the enclave is in debug mode
    fn is_debug(&self) -> bool;
    /// `get_key_manager` returns a key manager for Enclave Keys
    fn get_key_manager(&self) -> &EnclaveKeyManager;
    /// `with_ecall_permit` guards entry into enclave ECALLs.
    fn with_ecall_permit<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        f()
    }
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
    fn with_ecall_permit<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.ecall_gate.with_permit(f)
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
    /// canonical store still matches the explicit base state that seeded the batch.
    /// The check and apply run under the same serialized update transaction keyed by
    /// `update_key`, so the canonical base cannot change between verification and commit.
    fn apply_write_set_with_expected_base(
        &self,
        update_key: UpdateKey,
        prev_height: Height,
        client_state: &Any,
        consensus_state: &Any,
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
        client_state: &Any,
        consensus_state: &Any,
    ) -> Result<()>
    where
        S: TxAccessor,
    {
        let client_state_key = store_key::client_state_bytes(client_id);
        let client_state_value =
            bincode::serde::encode_to_vec(client_state, bincode::config::standard())
                .map_err(Error::bincode_encode)?;
        let canonical_client_state =
            self.use_mut_store(|store| store.tx_get(tx_id, &client_state_key))?;
        if canonical_client_state.as_deref() != Some(client_state_value.as_slice()) {
            return Err(Error::invalid_argument(format!(
                "canonical speculative base client_state mismatch: client_id={}",
                client_id
            )));
        }

        let consensus_state_key = store_key::consensus_state_bytes(client_id, prev_height);
        let consensus_state_value =
            bincode::serde::encode_to_vec(consensus_state, bincode::config::standard())
                .map_err(Error::bincode_encode)?;
        let canonical_consensus_state =
            self.use_mut_store(|store| store.tx_get(tx_id, &consensus_state_key))?;
        if canonical_consensus_state.as_deref() != Some(consensus_state_value.as_slice()) {
            return Err(Error::invalid_argument(format!(
                "canonical speculative base consensus_state mismatch: client_id={} height={}-{}",
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

#[cfg(test)]
mod tests {
    use super::ECallGate;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn ecall_gate_limits_concurrency() {
        let gate = Arc::new(ECallGate::new(2));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..6 {
            let gate = gate.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                gate.with_permit(|| {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    observed_max.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(25));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(observed_max.load(Ordering::SeqCst), 2);
    }
}

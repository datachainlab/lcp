use crate::errors::{Error, Result};
use keymanager::EnclaveKeyManager;
use lcp_types::{store_key, EnclaveMetadata, Height};
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
    /// store already contains the explicit historical state ID that seeded the
    /// batch at `prev_height`.
    ///
    /// The check and apply run under the same serialized update transaction keyed by
    /// `update_key`, so the accepted base cannot change between verification and
    /// commit.
    ///
    /// This intentionally does not byte-compare the explicit base client state
    /// or consensus state with the local store: the on-chain/client protocol
    /// path is allowed to start from a historical base, and raw `Any` encodings
    /// are not the canonical representation of a light-client state. The local
    /// check anchors the supplied base by the state commitment that the
    /// in-enclave light client actually observed: the caller-supplied
    /// `prev_state_id` (recorded by the first speculative unit) must match the
    /// height-indexed state ID previously stored by a successful
    /// create/serial/speculative update.
    ///
    /// The on-chain state ID chain is the source of truth for finality: a
    /// signed update is accepted only if its `prev_state_id` matches the
    /// on-chain client's current state. The host store's latest `clientState`
    /// is therefore a local cache/cursor, not the authoritative commit point
    /// for speculative batches. This host-side check is deliberately narrower:
    /// it prevents signing from an entirely unknown base by requiring
    /// continuity from a state ID that LCP has already observed and stored.
    fn apply_write_set_with_expected_base(
        &self,
        update_key: UpdateKey,
        prev_height: Height,
        prev_state_id: Option<&[u8]>,
        write_set: WriteSet,
    ) -> Result<()>
    where
        S: TxAccessor,
    {
        let tx = self.begin_tx(Some(update_key.clone()))?;
        let tx_id = tx.get_id();
        if let Err(e) =
            self.verify_expected_base_state_in_tx(tx_id, &update_key, &prev_height, prev_state_id)
        {
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
        prev_state_id: Option<&[u8]>,
    ) -> Result<()>
    where
        S: TxAccessor,
    {
        let prev_state_id = prev_state_id.ok_or_else(|| {
            Error::invalid_argument(format!(
                "speculative update_client must provide prev_state_id: client_id={} height={}-{}",
                client_id,
                prev_height.revision_number(),
                prev_height.revision_height()
            ))
        })?;
        // Do not recompute the state ID from the supplied raw Anys, and do not
        // compare those raw bytes against stored client/consensus states here.
        // The light-client-specific canonicalization that defines state_id is
        // available only inside the enclave. The first speculative unit reports
        // the state_id it observed after seeding the supplied base, and this
        // stored state_id was written by the same in-enclave light client at a
        // previous create/update commit. Comparing the two is therefore the
        // canonical base-connection check without encoding-only false
        // mismatches.
        //
        // Note the intentionally limited authority of this check. The LCP host
        // store is not the SSOT for whether this update can land; the
        // destination chain's state ID is. Here we only ensure the proposed
        // speculative chain starts from a state ID that LCP knows about. If the
        // on-chain client has already moved past that state, the on-chain
        // verifier rejects the signed message by state ID; the local latest
        // clientState cache is not used as a commit-time CAS.
        let state_id_key = store_key::state_id_bytes(client_id, prev_height);
        let stored_state_id = self.use_mut_store(|store| store.tx_get(tx_id, &state_id_key))?;
        let Some(stored_state_id) = stored_state_id else {
            return Err(Error::invalid_argument(format!(
                "stored speculative base state_id missing: client_id={} height={}-{}; run a serial update_client once to populate state_id tracking before retrying explicit-state batch execution",
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

use crate::transaction::{CommitStore, CreatedTx, Tx, TxAccessor, UpdateKey};
use crate::{Error, KVStore, Result, TxId};
use core::marker::PhantomData;
use log::*;
use ouroboros::self_referencing;
use rocksdb::{
    Error as RocksDBError, SnapshotWithThreadMode, Transaction, TransactionDB, TransactionOptions,
    WriteOptions, DB,
};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

/// `RocksDBStore` is a store implementation with RocksDB
#[self_referencing]
pub struct RocksDBStore {
    db: InnerDB,
    latest_tx_id: TxId,
    #[borrows(db)]
    #[covariant]
    txs: HashMap<TxId, StoreTransaction<'this>>,
    mutex: HashMap<UpdateKey, Rc<Mutex<()>>>,
}

unsafe impl Send for RocksDBStore {}
unsafe impl Sync for RocksDBStore {}

impl RocksDBStore {
    pub fn create(db: TransactionDB) -> Self {
        RocksDBStoreBuilder {
            db: InnerDB::TransactionDB(db),
            latest_tx_id: Default::default(),
            txs_builder: |_| Default::default(),
            mutex: Default::default(),
        }
        .build()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        Self::create(TransactionDB::open_default(path).unwrap())
    }

    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Self {
        let db = DB::open_for_read_only(&Default::default(), path, false).unwrap();
        RocksDBStoreBuilder {
            db: InnerDB::ReadOnlyDB(db),
            latest_tx_id: Default::default(),
            txs_builder: |_| Default::default(),
            mutex: Default::default(),
        }
        .build()
    }

    pub fn finalize_tx<T>(
        &mut self,
        tx: RocksDBTx<PreparedRocksDBTx>,
        f: impl FnOnce(StoreTransaction) -> T,
    ) -> T {
        self.with_mut(|fields| {
            if tx.is_update_tx() {
                let update_key = tx.borrow_update_key().as_ref().unwrap();
                let v = fields.mutex.get(update_key).expect("invariant violation");
                if Rc::strong_count(v) == 2 {
                    // "2" indicates `v` and an entry of `mutex` only exist
                    // so, remove the entry
                    fields.mutex.remove_entry(update_key);
                }
            }
            let (_, tx) = fields
                .txs
                .remove_entry(tx.borrow_id())
                .expect("tx not found");
            f(tx)
        })
    }
}

impl KVStore for RocksDBStore {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.borrow_db().set(key, value).unwrap()
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.borrow_db().get(key)
    }

    fn remove(&mut self, key: &[u8]) {
        self.borrow_db().remove(key)
    }
}

impl TxAccessor for RocksDBStore {
    fn run_in_tx<T>(&self, tx_id: TxId, f: impl FnOnce(&dyn KVStore) -> T) -> Result<T> {
        self.with_txs(|txs| {
            let stx = txs
                .get(&tx_id)
                .ok_or_else(|| Error::tx_id_not_found(tx_id))?;
            Ok(f(stx))
        })
    }

    fn run_in_mut_tx<T>(
        &mut self,
        tx_id: TxId,
        f: impl FnOnce(&mut dyn KVStore) -> T,
    ) -> Result<T> {
        self.with_txs_mut(|txs| {
            let stx = txs
                .get_mut(&tx_id)
                .ok_or_else(|| Error::tx_id_not_found(tx_id))?;
            Ok(f(stx))
        })
    }
}

impl CommitStore for RocksDBStore {
    type Tx = RocksDBTx<CreatedRocksDBTx>;

    fn create_transaction(&mut self, update_key: Option<UpdateKey>) -> Result<Self::Tx> {
        debug!("create tx: {:?}", update_key);
        self.with_mut(|fields| {
            fields.latest_tx_id.safe_incr()?;

            match fields.db {
                InnerDB::ReadOnlyDB(_) => {
                    // NOTE: ignore `update_key`
                    Ok(RocksDBTx::new_read_tx(*fields.latest_tx_id))
                }
                InnerDB::TransactionDB(_) => {
                    if let Some(update_key) = update_key {
                        if update_key.is_empty() {
                            return Err(Error::invalid_update_key_length(0));
                        }
                        if !fields.mutex.contains_key(&update_key) {
                            let mutex = Rc::new(Mutex::new(()));
                            let _ = fields.mutex.insert(update_key.clone(), mutex);
                        }
                        let mutex = fields.mutex.get(&update_key).unwrap().clone();
                        Ok(RocksDBTx::new_update_tx(
                            *fields.latest_tx_id,
                            update_key,
                            mutex,
                        ))
                    } else {
                        Ok(RocksDBTx::new_read_tx(*fields.latest_tx_id))
                    }
                }
            }
        })
    }

    fn begin(&mut self, tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        debug!("begin tx: {:?}", tx.get_id());
        self.with_mut(|fields| {
            let mut tx_opt = TransactionOptions::default();
            tx_opt.set_snapshot(true);

            let stx = match fields.db {
                InnerDB::ReadOnlyDB(db) => StoreTransaction::ReadSnapshot(ReadSnapshot {
                    snapshot: db.snapshot(),
                    buffer: HashMap::default(),
                }),
                InnerDB::TransactionDB(db) => {
                    if tx.is_update_tx() {
                        StoreTransaction::Update(
                            UpdateTransactionBuilder {
                                tx: db.transaction_opt(&WriteOptions::default(), &tx_opt),
                                snapshot_builder: |tx| tx.snapshot(),
                            }
                            .build(),
                        )
                    } else {
                        StoreTransaction::Read(ReadTransaction {
                            snapshot: db.snapshot(),
                            buffer: HashMap::default(),
                        })
                    }
                }
            };

            fields.txs.insert(tx.get_id(), stx);
            Ok(())
        })
    }

    fn commit(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        debug!("commit tx: {:?}", tx.get_id());
        self.finalize_tx(tx, |stx| stx.commit())
    }

    fn rollback(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) {
        debug!("rollback tx: {:?}", tx.get_id());
        self.finalize_tx(tx, |stx| stx.rollback())
    }
}

/// InnerDB defines multiple DB types
pub enum InnerDB {
    TransactionDB(TransactionDB),
    ReadOnlyDB(DB),
}

impl InnerDB {
    pub(crate) fn set(
        &self,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> core::result::Result<(), RocksDBError> {
        match self {
            Self::TransactionDB(db) => db.put(key, value),
            Self::ReadOnlyDB(db) => db.put(key, value),
        }
    }

    pub(crate) fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::TransactionDB(db) => db.get(key).unwrap(),
            Self::ReadOnlyDB(db) => db.get(key).unwrap(),
        }
    }

    pub(crate) fn remove(&self, key: &[u8]) {
        match self {
            Self::TransactionDB(db) => db.delete(key).unwrap(),
            Self::ReadOnlyDB(db) => db.delete(key).unwrap(),
        }
    }
}

/// StoreTransaction implements multiple transaction types
pub enum StoreTransaction<'a> {
    Read(ReadTransaction<'a>),
    Update(UpdateTransaction<'a>),
    ReadSnapshot(ReadSnapshot<'a>),
}

#[allow(clippy::single_match)]
impl<'a> StoreTransaction<'a> {
    fn commit(self) -> Result<()> {
        match self {
            StoreTransaction::Update(stx) => stx.commit(),
            _ => Ok(()),
        }
    }

    fn rollback(&self) {
        match self {
            StoreTransaction::Update(stx) => stx.rollback(),
            _ => {}
        }
    }
}

impl<'a> KVStore for StoreTransaction<'a> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        match self {
            StoreTransaction::Read(stx) => stx.set(key, value),
            StoreTransaction::Update(stx) => stx.set(key, value),
            StoreTransaction::ReadSnapshot(stx) => stx.set(key, value),
        }
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self {
            StoreTransaction::Read(stx) => stx.get(key),
            StoreTransaction::Update(stx) => stx.get(key),
            StoreTransaction::ReadSnapshot(stx) => stx.get(key),
        }
    }

    fn remove(&mut self, key: &[u8]) {
        match self {
            StoreTransaction::Read(stx) => stx.remove(key),
            StoreTransaction::Update(stx) => stx.remove(key),
            StoreTransaction::ReadSnapshot(stx) => stx.remove(key),
        }
    }
}

/// ReadTransaction is a `read-only` transaction.
/// All read operations are performed based on a specific version of snapshot.
/// All write operations are applied to the transaction's buffer, but they are never committed to the DB.
pub struct ReadTransaction<'a> {
    snapshot: SnapshotWithThreadMode<'a, TransactionDB>,
    buffer: HashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl<'a> KVStore for ReadTransaction<'a> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.buffer.insert(key, Some(value));
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self.buffer.get(key) {
            Some(Some(v)) => Some(v.to_vec()),
            Some(None) => None, // already removed in the tx
            None => self.snapshot.get(key).unwrap(),
        }
    }

    fn remove(&mut self, key: &[u8]) {
        self.buffer.insert(key.to_vec(), None);
    }
}

/// UpdateTransaction is a `writable` transaction
/// All read operations are performed based on a specific version of snapshot.
/// All write operations are applied to the corresponding RocksDB's transaction
#[self_referencing]
pub struct UpdateTransaction<'a> {
    tx: Transaction<'a, TransactionDB>,
    #[borrows(tx)]
    #[covariant]
    snapshot: SnapshotWithThreadMode<'this, Transaction<'this, TransactionDB>>,
}

impl<'a> UpdateTransaction<'a> {
    fn commit(self) -> Result<()> {
        self.into_heads()
            .tx
            .commit()
            .map_err(|e| Error::commit_tx(e.into_string()))
    }

    fn rollback(&self) {
        self.with_tx(|tx| tx.rollback()).unwrap()
    }
}

impl<'a> KVStore for UpdateTransaction<'a> {
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.with_tx(|tx| tx.put(k, v)).unwrap()
    }

    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        self.with_snapshot(|snapshot| snapshot.get(k).unwrap())
    }

    fn remove(&mut self, key: &[u8]) {
        self.with_tx(|tx| tx.delete(key)).unwrap()
    }
}

/// ReadSnapshot is a `read-only` transaction.
/// All read operations are performed based on a specific version of snapshot.
/// All write operations are applied to the transaction's buffer, but they are never committed to the DB.
pub struct ReadSnapshot<'a> {
    snapshot: SnapshotWithThreadMode<'a, DB>,
    buffer: HashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl<'a> KVStore for ReadSnapshot<'a> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.buffer.insert(key, Some(value));
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self.buffer.get(key) {
            Some(Some(v)) => Some(v.to_vec()),
            Some(None) => None, // already removed in the tx
            None => self.snapshot.get(key).unwrap(),
        }
    }

    fn remove(&mut self, key: &[u8]) {
        self.buffer.insert(key.to_vec(), None);
    }
}

/// RocksDBTx is a transaction handle corresponding to `StoreTransaction`
#[self_referencing]
pub struct RocksDBTx<T> {
    pub id: TxId,
    pub update_key: Option<UpdateKey>,
    pub mutex: Option<Rc<Mutex<()>>>,
    #[borrows(mutex)]
    #[covariant]
    pub mutex_guard: Option<MutexGuard<'this, ()>>,
    marker: PhantomData<T>,
}

/// CreatedRocksDBTx represents a type of transaction that cannot be begun yet
pub struct CreatedRocksDBTx;

/// PreparedRocksDBTx represents a type of transaction that has been begun or can be begun
pub struct PreparedRocksDBTx;

impl<T> Tx for RocksDBTx<T> {
    fn get_id(&self) -> TxId {
        *self.borrow_id()
    }
}

impl CreatedTx for RocksDBTx<CreatedRocksDBTx> {
    type PreparedTx = RocksDBTx<PreparedRocksDBTx>;

    fn prepare(self) -> Result<Self::PreparedTx> {
        let update = self.is_update_tx();
        let fields = self.into_heads();
        let tx = RocksDBTxBuilder {
            id: fields.id,
            update_key: fields.update_key,
            mutex: fields.mutex,
            mutex_guard_builder: |m| {
                if update {
                    Some(
                        m.as_ref()
                            .unwrap()
                            .lock()
                            .map_err(|e| Error::wait_mutex(e.to_string()))
                            .unwrap(),
                    )
                } else {
                    None
                }
            },
            marker: Default::default(),
        }
        .build();
        Ok(tx)
    }
}

impl<T> RocksDBTx<T> {
    pub fn new_read_tx(id: TxId) -> Self {
        RocksDBTxBuilder {
            id,
            update_key: None,
            mutex: None,
            mutex_guard_builder: |_| None,
            marker: Default::default(),
        }
        .build()
    }

    pub fn new_update_tx(id: TxId, update_key: UpdateKey, mutex: Rc<Mutex<()>>) -> Self {
        RocksDBTxBuilder {
            id,
            update_key: Some(update_key),
            mutex: Some(mutex),
            mutex_guard_builder: |_| None,
            marker: Default::default(),
        }
        .build()
    }

    pub fn is_update_tx(&self) -> bool {
        self.borrow_update_key().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use core::time::Duration;
    use std::{
        collections::HashSet,
        sync::{Condvar, RwLock},
        thread,
        time::SystemTime,
    };
    use tempfile::TempDir;

    #[test]
    fn test_store() {
        let _ = env_logger::try_init();
        let tmp_dir = TempDir::new().unwrap();
        let mut store = RocksDBStore::open(tmp_dir.as_ref());

        // case1: set key-value pair simply in update tx
        // pre:  initial state
        // post: k0 -> v0
        {
            let tx = store.create_transaction(Some("test".into())).unwrap();
            assert_eq!(store.borrow_mutex().len(), 1);
            let tx = tx.prepare().unwrap();
            store.begin(&tx).unwrap();
            assert!(store.tx_set(tx.get_id(), key(0), value(0)).is_ok());
            assert!(store
                .tx_get(tx.get_id(), &key(0))
                .unwrap()
                .eq(&Some(value(0))));
            store.commit(tx).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
        }

        // case2: get key-value pair simply in read tx
        // post: k0 -> v0
        {
            let tx = store.create_transaction(None).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
            let tx = tx.prepare().unwrap();
            store.begin(&tx).unwrap();
            assert!(store
                .tx_get(tx.get_id(), &key(0))
                .unwrap()
                .eq(&Some(value(0))));
            assert!(store.tx_set(tx.get_id(), key(0), value(1)).is_ok());
            assert!(store
                .tx_get(tx.get_id(), &key(0))
                .unwrap()
                .eq(&Some(value(1))));
            store.commit(tx).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
        }

        // case3: remove key-value pair simply in read tx
        // post: k0 -> v0
        {
            let tx = store.create_transaction(None).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
            let tx = tx.prepare().unwrap();
            store.begin(&tx).unwrap();
            store.tx_remove(tx.get_id(), &key(0)).unwrap();
            assert!(store.tx_get(tx.get_id(), &key(0)).unwrap().eq(&None));
            store.commit(tx).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
            assert!(store.get(&key(0)).ne(&None));
        }

        // case4: remove key-value pair simply in update tx
        // post: empty
        {
            let tx = store.create_transaction(Some("test".into())).unwrap();
            assert_eq!(store.borrow_mutex().len(), 1);
            let tx = tx.prepare().unwrap();
            store.begin(&tx).unwrap();
            store.tx_remove(tx.get_id(), &key(0)).unwrap();
            assert!(store.tx_get(tx.get_id(), &key(0)).unwrap().eq(&None));
            store.commit(tx).unwrap();
            assert_eq!(store.borrow_mutex().len(), 0);
            assert!(store.get(&key(0)).eq(&None));
        }

        // case5: set key-value pair but rollback it
        {
            let tx = store.create_transaction(Some("test".into())).unwrap();
            assert_eq!(store.borrow_mutex().len(), 1);
            let tx = tx.prepare().unwrap();
            store.begin(&tx).unwrap();
            assert!(store.tx_set(tx.get_id(), key(0), value(0)).is_ok());
            store.rollback(tx);
            assert_eq!(store.borrow_mutex().len(), 0);
            assert!(store.get(&key(0)).eq(&None));
        }
    }

    #[test]
    fn test_concurrent_write_tx_with_same_update_key_1() {
        let (_tmp_dir, store, [r1, r2]) = get_test_helpers::<2>(vec![]);

        // r1: create&prepare -> begin  -> commit
        //                     \                   \
        // r2:                   create -> prepare(blocking) -> begin&commit

        let th1 = thread::spawn(move || {
            r1.create(Some("test"))
                .emit_event(1)
                .prepare()
                .emit_event(2)
                .begin()
                .set(key_s(0), value_s(0))
                .commit()
        });

        let th2 = thread::spawn(move || {
            r2.block_on(1, 1)
                .create(Some("test"))
                .block_on(1, 2)
                .prepare()
                .begin()
                .get(key_s(0), Some(value_s(0)))
                .set(key_s(0), value_s(1))
                .commit()
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(1))));
    }

    #[test]
    fn test_concurrent_write_tx_with_same_update_key_2() {
        let (_tmp_dir, store, [r1, r2]) = get_test_helpers::<2>(vec![]);

        // r1:        create -> prepare -> begin -> commit
        //           /                                    \
        // r2: create --------> prepare(blocking) ---------> begin&commit

        let th1 = thread::spawn(move || {
            r1.block_on(2, 1)
                .create(Some("test"))
                .prepare()
                .emit_event(1)
                .begin()
                .set(key_s(0), value_s(0))
                .commit()
        });

        let th2 = thread::spawn(move || {
            r2.create(Some("test"))
                .emit_event(1)
                .block_on(1, 1)
                .prepare()
                .begin()
                .get(key_s(0), Some(value_s(0)))
                .set(key_s(0), value_s(1))
                .commit()
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(1))));
    }

    #[test]
    fn test_concurrent_read_tx() {
        let (_tmp_dir, store, runners) = get_test_helpers::<8>(vec![]);

        let mut ths = vec![];
        let start = SystemTime::now();
        for runner in runners.into_iter() {
            let th = thread::spawn(move || {
                runner
                    .create(None)
                    .prepare()
                    .begin()
                    .get(key_s(0), None)
                    .execute(|| thread::sleep(Duration::from_secs(1)))
                    .commit()
            });
            ths.push(th);
        }
        for th in ths.into_iter() {
            th.join().unwrap();
        }
        let end = SystemTime::now().duration_since(start).unwrap();
        assert!(Duration::from_secs(1) * 2 > end);
        assert_eq!(store.read().unwrap().borrow_mutex().len(), 0);
    }

    #[test]
    fn test_concurrent_include_rollback() {
        let (_tmp_dir, store, [r1, r2]) = get_test_helpers::<2>(vec![]);

        let th1 = thread::spawn(move || {
            r1.create(Some("test"))
                .prepare()
                .emit_event(1)
                .block_on(2, 1)
                .begin()
                .set(key_s(0), value_s(0))
                .commit()
        });

        let th2 = thread::spawn(move || {
            r2.create(Some("test"))
                .block_on(1, 1)
                .emit_event(1)
                .prepare()
                .begin()
                .get(key_s(0), Some(value_s(0)))
                .set(key_s(0), value_s(1))
                .rollback()
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(0))));
    }

    #[test]
    fn test_concurrent_write_different_update_keys() {
        let (_tmp_dir, store, runners) = get_test_helpers::<8>(vec![]);

        let mut ths = vec![];
        let start = SystemTime::now();
        for r in runners {
            let th = thread::spawn(move || {
                let key = key_s(r.id);
                r.create(Some(key.as_str()))
                    .prepare()
                    .begin()
                    .set(key, value_s(0))
                    .execute(|| thread::sleep(Duration::from_secs(1)))
                    .commit()
            });
            ths.push(th);
        }
        for th in ths.into_iter() {
            th.join().unwrap();
        }
        let end = SystemTime::now().duration_since(start).unwrap();
        assert!(Duration::from_secs(1) * 2 > end);
        assert_eq!(store.read().unwrap().borrow_mutex().len(), 0);
    }

    #[test]
    fn test_write_and_snapshot() {
        let (tmp_dir, store, [r1, r2, r3]) = get_test_helpers::<3>(vec![2, 3]);

        let th1 = thread::spawn(move || {
            r1.create(Some("test"))
                .emit_event(1)
                .prepare()
                .emit_event(2)
                .begin()
                .set(key_s(0), value_s(0))
                .commit()
        });

        let th2 = thread::spawn(move || {
            r2.block_on(1, 1)
                .create(Some("test"))
                .block_on(1, 2)
                .prepare()
                .begin()
                .get(key_s(0), None)
                .set(key_s(0), value_s(1))
                .commit()
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(0))));

        // snapshot does not reflect the update
        let th3 = thread::spawn(move || {
            r3.create(None)
                .prepare()
                .begin()
                .get(key_s(0), None)
                .commit()
        });
        th3.join().unwrap();

        // So, reopen the DB as snapshot
        let new_snapshot = Arc::new(RwLock::new(RocksDBStore::open_read_only(tmp_dir.path())));
        assert!(new_snapshot
            .read()
            .unwrap()
            .get(&key(0))
            .eq(&Some(value(0))));
    }

    fn key(idx: u64) -> Vec<u8> {
        key_s(idx).into_bytes()
    }

    fn value(idx: u64) -> Vec<u8> {
        value_s(idx).into_bytes()
    }

    fn key_s(idx: u64) -> String {
        format!("k{}", idx)
    }

    fn value_s(idx: u64) -> String {
        format!("v{}", idx)
    }

    fn get_test_helpers<const S: usize>(
        read_only_ids: Vec<usize>,
    ) -> (TempDir, Arc<RwLock<RocksDBStore>>, [TxRunner; S]) {
        let _ = env_logger::try_init();
        let tmp_dir = TempDir::new().unwrap();
        let store = Arc::new(RwLock::new(RocksDBStore::open(tmp_dir.path())));
        let r_store = Arc::new(RwLock::new(RocksDBStore::open_read_only(tmp_dir.path())));
        let cond = Arc::new(Condvar::new());
        let events = Arc::new(Mutex::new(HashSet::new()));

        let runners = {
            let mut arr: [std::mem::MaybeUninit<TxRunner>; S] =
                unsafe { std::mem::MaybeUninit::uninit().assume_init() };
            for (i, elem) in arr.iter_mut().enumerate() {
                if read_only_ids.contains(&(i + 1)) {
                    unsafe {
                        std::ptr::write(
                            elem.as_mut_ptr(),
                            TxRunner::new(
                                (i + 1) as u64,
                                r_store.clone(),
                                cond.clone(),
                                events.clone(),
                            ),
                        );
                    }
                } else {
                    unsafe {
                        std::ptr::write(
                            elem.as_mut_ptr(),
                            TxRunner::new(
                                (i + 1) as u64,
                                store.clone(),
                                cond.clone(),
                                events.clone(),
                            ),
                        );
                    }
                }
            }
            let ptr = &mut arr as *mut _ as *mut [TxRunner; S];
            let res = unsafe { ptr.read() };
            core::mem::forget(arr);
            res
        };
        (tmp_dir, store, runners)
    }

    struct TxRunner {
        id: u64,
        channel: EventChannel,
        store: Arc<RwLock<RocksDBStore>>,
        created_tx: Option<RocksDBTx<CreatedRocksDBTx>>,
        prepared_tx: Option<RocksDBTx<PreparedRocksDBTx>>,
    }

    unsafe impl Send for TxRunner {}
    unsafe impl Sync for TxRunner {}

    struct EventChannel {
        self_rid: u64,
        cond: Arc<Condvar>,
        events: Arc<Mutex<HashSet<(u64, u64)>>>, // (rid, eid)
    }

    impl EventChannel {
        fn emit_event(&self, eid: u64) {
            debug!("emit: runner_id={} eid={}", self.self_rid, eid);
            self.events.lock().unwrap().insert((self.self_rid, eid));
            self.cond.notify_all();
        }
        #[allow(unused_must_use)]
        fn block_on(&self, rid: u64, eid: u64) {
            let events = self.events.lock().unwrap();
            self.cond
                .wait_while(events, |events| {
                    debug!(
                        "wait: self_runner_id={} target_runner_id={} eid={}",
                        self.self_rid, rid, eid
                    );
                    !events.contains(&(rid, eid))
                })
                .unwrap();
        }
    }

    impl TxRunner {
        fn new(
            id: u64,
            store: Arc<RwLock<RocksDBStore>>,
            cond: Arc<Condvar>,
            events: Arc<Mutex<HashSet<(u64, u64)>>>,
        ) -> Self {
            Self {
                id,
                channel: EventChannel {
                    self_rid: id,
                    cond,
                    events,
                },
                store,
                created_tx: None,
                prepared_tx: None,
            }
        }

        fn emit_event(self, eid: u64) -> Self {
            self.channel.emit_event(eid);
            self
        }

        fn block_on(self, rid: u64, eid: u64) -> Self {
            self.channel.block_on(rid, eid);
            self
        }

        fn create(mut self, update_key: Option<&str>) -> Self {
            debug!("create: id={} update_key={:?}", self.id, update_key);
            let tx = self
                .store
                .write()
                .unwrap()
                .create_transaction(update_key.map(|s| s.into()))
                .unwrap();
            self.created_tx = Some(tx);
            self
        }

        fn prepare(mut self) -> Self {
            debug!("prepare: id={}", self.id);
            self.prepared_tx = Some(self.created_tx.take().unwrap().prepare().unwrap());
            self
        }

        fn begin(self) -> Self {
            debug!("begin: id={}", self.id);
            self.store
                .write()
                .unwrap()
                .begin(self.prepared_tx.as_ref().unwrap())
                .unwrap();
            self
        }

        fn get<S: Into<String>>(self, key: S, expected_value: Option<S>) -> Self {
            let v = self
                .store
                .read()
                .unwrap()
                .tx_get(
                    self.prepared_tx.as_ref().unwrap().get_id(),
                    (key.into() as String).as_bytes(),
                )
                .unwrap();

            assert_eq!(
                v.map(|s| String::from_utf8(s).unwrap()),
                expected_value.map(|s| s.into())
            );
            self
        }

        fn set<S: Into<String>>(self, key: S, value: S) -> Self {
            self.store
                .write()
                .unwrap()
                .tx_set(
                    self.prepared_tx.as_ref().unwrap().get_id(),
                    (key.into() as String).into_bytes(),
                    (value.into() as String).into_bytes(),
                )
                .unwrap();
            self
        }

        fn execute(self, f: impl FnOnce()) -> Self {
            debug!("execute: id={}", self.id);
            f();
            self
        }

        fn commit(self) {
            debug!("commit: id={}", self.id);
            let tx = self.prepared_tx.unwrap();
            self.store.write().unwrap().commit(tx).unwrap();
        }

        fn rollback(self) {
            debug!("rollback: id={}", self.id);
            let tx = self.prepared_tx.unwrap();
            self.store.write().unwrap().rollback(tx);
        }
    }
}

use crate::transaction::{CommitStore, CreatedTx, Tx, TxAccessor, UpdateKey};
use crate::{Error, KVStore, Result, TxId};
use core::marker::PhantomData;
use ouroboros::self_referencing;
use rocksdb::{
    SnapshotWithThreadMode, Transaction, TransactionDB, TransactionOptions, WriteOptions,
};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

/// `RocksDBStore` is a store implementation with RocksDB
#[self_referencing]
pub struct RocksDBStore {
    db: TransactionDB,
    latest_tx_id: TxId,
    #[borrows(db)]
    #[covariant]
    txs: HashMap<TxId, StoreTransaction<'this>>,
    mutex: HashMap<UpdateKey, (Rc<Mutex<()>>, u64)>,
}

unsafe impl Send for RocksDBStore {}
unsafe impl Sync for RocksDBStore {}

impl RocksDBStore {
    pub fn create(db: TransactionDB) -> Self {
        RocksDBStoreBuilder {
            db,
            latest_tx_id: Default::default(),
            txs_builder: |_| Default::default(),
            mutex: Default::default(),
        }
        .build()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        Self::create(TransactionDB::open_default(path).unwrap())
    }

    pub fn finalize_tx<T>(
        &mut self,
        tx: RocksDBTx<PreparedRocksDBTx>,
        f: impl FnOnce(StoreTransaction) -> T,
    ) -> T {
        self.with_mut(|fields| {
            if tx.is_update_tx() {
                let update_key = tx.borrow_update_key().as_ref().unwrap();
                let v = fields
                    .mutex
                    .get_mut(update_key)
                    .expect("invariant violation");
                assert!(v.1 > 0);
                if v.1 == 1 {
                    fields.mutex.remove_entry(update_key);
                } else {
                    v.1 -= 1;
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
        self.borrow_db().put(key, value).unwrap()
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.borrow_db().get(key).unwrap()
    }

    fn remove(&mut self, key: &[u8]) {
        self.borrow_db().delete(key).unwrap()
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
        self.with_mut(|fields| {
            fields.latest_tx_id.safe_incr()?;
            if let Some(update_key) = update_key {
                if update_key.len() == 0 {
                    return Err(Error::invalid_update_key_length(0));
                }
                if !fields.mutex.contains_key(&update_key) {
                    let mutex = Rc::new(Mutex::new(()));
                    let _ = fields.mutex.insert(update_key.clone(), (mutex.clone(), 0));
                }
                let mutex = fields.mutex.get(&update_key).unwrap().0.clone();
                fields.mutex.get_mut(&update_key).unwrap().1 += 1;
                Ok(RocksDBTx::new_update_tx(
                    *fields.latest_tx_id,
                    update_key,
                    mutex,
                ))
            } else {
                Ok(RocksDBTx::new_read_tx(*fields.latest_tx_id))
            }
        })
    }

    fn begin(&mut self, tx: &<Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.with_mut(|fields| {
            let mut tx_opt = TransactionOptions::default();
            tx_opt.set_snapshot(true);

            let stx = if tx.is_update_tx() {
                StoreTransaction::Update(
                    UpdateTransactionBuilder {
                        tx: fields.db.transaction_opt(&WriteOptions::default(), &tx_opt),
                        snapshot_builder: |tx| tx.snapshot(),
                    }
                    .build(),
                )
            } else {
                let snapshot = fields.db.snapshot();
                StoreTransaction::Read(ReadTransaction {
                    snapshot,
                    buffer: HashMap::default(),
                })
            };
            log::info!("begin tx: {:?}", tx.get_id());
            fields.txs.insert(tx.get_id(), stx);
            Ok(())
        })
    }

    fn commit(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) -> Result<()> {
        self.finalize_tx(tx, |stx| stx.commit())
    }

    fn rollback(&mut self, tx: <Self::Tx as CreatedTx>::PreparedTx) {
        self.finalize_tx(tx, |stx| stx.rollback())
    }
}

/// StoreTransaction implements two transaction types
pub enum StoreTransaction<'a> {
    Read(ReadTransaction<'a>),
    Update(UpdateTransaction<'a>),
}

impl<'a> StoreTransaction<'a> {
    fn commit(self) -> Result<()> {
        match self {
            StoreTransaction::Read(stx) => stx.commit(),
            StoreTransaction::Update(stx) => stx.commit(),
        }
    }

    fn rollback(&self) {
        match self {
            StoreTransaction::Read(stx) => stx.rollback(),
            StoreTransaction::Update(stx) => stx.rollback(),
        }
    }
}

impl<'a> KVStore for StoreTransaction<'a> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        match self {
            StoreTransaction::Read(stx) => stx.set(key, value),
            StoreTransaction::Update(stx) => stx.set(key, value),
        }
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self {
            StoreTransaction::Read(stx) => stx.get(key),
            StoreTransaction::Update(stx) => stx.get(key),
        }
    }

    fn remove(&mut self, key: &[u8]) {
        match self {
            StoreTransaction::Read(stx) => stx.remove(key),
            StoreTransaction::Update(stx) => stx.remove(key),
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

impl<'a> ReadTransaction<'a> {
    fn commit(self) -> Result<()> {
        Ok(())
    }

    fn rollback(&self) {}
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
        self.borrow_id().clone()
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
    use alloc::{collections::VecDeque, sync::Arc};
    use core::time::Duration;
    use log::*;
    use std::{
        sync::{Condvar, RwLock},
        thread,
        time::SystemTime,
    };
    use tempdir::TempDir;

    #[test]
    fn test_store() {
        let tmp_dir = TempDir::new("store-rocksdb").unwrap().into_path();
        let mut db = RocksDBStore::open(tmp_dir.clone());

        // case1: set key-value pair simply in update tx
        // pre:  initial state
        // post: k0 -> v0
        {
            let tx = db.create_transaction(Some("test".into())).unwrap();
            assert!(db.borrow_mutex().len() == 1);
            let tx = tx.prepare().unwrap();
            db.begin(&tx).unwrap();
            db.tx_set(tx.get_id(), key(0), value(0)).unwrap();
            assert!(db.tx_get(tx.get_id(), &key(0)).unwrap().eq(&Some(value(0))));
            db.commit(tx).unwrap();
            assert!(db.borrow_mutex().len() == 0);
        }

        // case2: get key-value pair simply in read tx
        // post: k0 -> v0
        {
            let tx = db.create_transaction(None).unwrap();
            assert!(db.borrow_mutex().len() == 0);
            let tx = tx.prepare().unwrap();
            db.begin(&tx).unwrap();
            assert!(db.tx_get(tx.get_id(), &key(0)).unwrap().eq(&Some(value(0))));
            db.commit(tx).unwrap();
            assert!(db.borrow_mutex().len() == 0);
        }

        // case3: remove key-value pair simply in read tx
        // post: k0 -> v0
        {
            let tx = db.create_transaction(None).unwrap();
            assert!(db.borrow_mutex().len() == 0);
            let tx = tx.prepare().unwrap();
            db.begin(&tx).unwrap();
            db.tx_remove(tx.get_id(), &key(0)).unwrap();
            assert!(db.tx_get(tx.get_id(), &key(0)).unwrap().eq(&None));
            db.commit(tx).unwrap();
            assert!(db.borrow_mutex().len() == 0);
            assert!(db.get(&key(0)).ne(&None));
        }

        // case4: remove key-value pair simply in update tx
        // post: empty
        {
            let tx = db.create_transaction(Some("test".into())).unwrap();
            assert!(db.borrow_mutex().len() == 1);
            let tx = tx.prepare().unwrap();
            db.begin(&tx).unwrap();
            db.tx_remove(tx.get_id(), &key(0)).unwrap();
            assert!(db.tx_get(tx.get_id(), &key(0)).unwrap().eq(&None));
            db.commit(tx).unwrap();
            assert!(db.borrow_mutex().len() == 0);
            assert!(db.get(&key(0)).eq(&None));
        }
    }

    #[test]
    fn test_concurrent_write_tx_with_same_update_key() {
        let (_tmp_dir, store, mut runners) = get_test_helpers(2);
        let r1 = runners.pop_front().unwrap();
        let r2 = runners.pop_front().unwrap();

        let th1 = thread::spawn(move || {
            r1.create(Some("test"))
                .prepare()
                .emit_event(1)
                .block_on(2, 1)
                .begin()
                .set("k0", "v0")
                .commit()
        });

        let th2 = thread::spawn(move || {
            r2.create(Some("test"))
                .block_on(1, 1)
                .emit_event(1)
                .prepare()
                .begin()
                .get("k0", Some("v0"))
                .set("k0", "v1")
                .commit()
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(1))));
    }

    #[test]
    fn test_concurrent_read_tx() {
        let (_tmp_dir, _store, runners) = get_test_helpers(8);

        let mut ths = vec![];
        let start = SystemTime::now();
        for runner in runners.into_iter() {
            let th = thread::spawn(move || {
                runner
                    .create(None)
                    .prepare()
                    .begin()
                    .get("k0", None)
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
    }

    fn key(idx: u32) -> Vec<u8> {
        format!("k{}", idx).into_bytes()
    }

    fn value(idx: u32) -> Vec<u8> {
        format!("v{}", idx).into_bytes()
    }

    fn get_test_helpers(size: u64) -> (TempDir, Arc<RwLock<RocksDBStore>>, VecDeque<TxRunner>) {
        let _ = env_logger::try_init();
        let tmp_dir = TempDir::new("store-rocksdb").unwrap();
        let store = Arc::new(RwLock::new(RocksDBStore::open(tmp_dir.path())));
        let cond = Arc::new(Condvar::new());
        let events = Arc::new(Mutex::new(HashMap::new()));
        let mut runners = VecDeque::<TxRunner>::default();
        for i in 1..=size {
            runners.push_back(TxRunner::new(
                i,
                store.clone(),
                cond.clone(),
                events.clone(),
            ));
        }
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
        self_id: u64,
        cond: Arc<Condvar>,
        events: Arc<Mutex<HashMap<u64, u64>>>,
    }

    impl EventChannel {
        fn emit_event(&self, eid: u64) {
            info!("emit: teid={} eid={}", self.self_id, eid);
            self.events.lock().unwrap().insert(self.self_id, eid);
            self.cond.notify_all();
            info!("done notification: eid={}", eid);
        }
        fn block_on(&self, teid: u64, eid: u64) {
            let events = self.events.lock().unwrap();
            let _ = self
                .cond
                .wait_while(events, |events| {
                    log::info!("wakeup: wait={} {}", teid, eid);
                    !match events.get(&teid) {
                        Some(v) => eid == *v,
                        None => false,
                    }
                })
                .unwrap();
        }
    }

    impl TxRunner {
        fn new(
            id: u64,
            store: Arc<RwLock<RocksDBStore>>,
            cond: Arc<Condvar>,
            events: Arc<Mutex<HashMap<u64, u64>>>,
        ) -> Self {
            Self {
                id,
                channel: EventChannel {
                    self_id: id,
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

        fn block_on(self, teid: u64, eid: u64) -> Self {
            self.channel.block_on(teid, eid);
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
            f();
            self
        }

        fn commit(self) {
            log::info!("commit: id={}", self.id);
            let tx = self.prepared_tx.unwrap();
            self.store.write().unwrap().commit(tx).unwrap();
        }
    }
}

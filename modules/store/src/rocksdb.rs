use crate::transaction::{CommitStore, CreatedTx, Tx, TxStore, UpdateKey};
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

impl TxStore for RocksDBStore {
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

pub struct CreatedRocksDBTx;
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
    use alloc::sync::Arc;
    use core::time::Duration;
    use log::*;
    use std::{sync::RwLock, thread};
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
    fn test_two_concurrent_tx() {
        env_logger::init();

        let tmp_dir = TempDir::new("store-rocksdb").unwrap().into_path();
        let store = Arc::new(RwLock::new(RocksDBStore::open(tmp_dir.clone())));

        let th1 = {
            let store = store.clone();
            thread::spawn(move || {
                debug!("th1 start");
                let tx = store
                    .write()
                    .unwrap()
                    .create_transaction(Some("test".into()))
                    .unwrap();
                let tx = tx.prepare().unwrap();
                store.write().unwrap().begin(&tx).unwrap();
                debug!("th1 sleep");
                thread::sleep(Duration::from_millis(20));
                debug!("th1 wakeup");
                store
                    .write()
                    .unwrap()
                    .tx_set(tx.get_id(), key(0), value(0))
                    .unwrap();
                store.write().unwrap().commit(tx).unwrap();
                debug!("th1 end");
            })
        };

        thread::sleep(Duration::from_millis(10));

        let th2 = {
            let store = store.clone();
            thread::spawn(move || {
                debug!("th2 start");
                let tx = store
                    .write()
                    .unwrap()
                    .create_transaction(Some("test".into()))
                    .unwrap();
                let tx = tx.prepare().unwrap();
                store.write().unwrap().begin(&tx).unwrap();
                store
                    .write()
                    .unwrap()
                    .tx_set(tx.get_id(), key(0), value(1))
                    .unwrap();
                store.write().unwrap().commit(tx).unwrap();
                debug!("th2 end");
            })
        };

        th1.join().unwrap();
        th2.join().unwrap();
        assert!(store.read().unwrap().get(&key(0)).eq(&Some(value(1))));
    }

    fn key(idx: u32) -> Vec<u8> {
        format!("k{}", idx).into_bytes()
    }

    fn value(idx: u32) -> Vec<u8> {
        format!("v{}", idx).into_bytes()
    }
}

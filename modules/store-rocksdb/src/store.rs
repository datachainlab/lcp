use ouroboros::self_referencing;
use rocksdb::{
    SnapshotWithThreadMode, Transaction, TransactionDB, TransactionOptions, WriteOptions,
};
use std::collections::BTreeMap;
use std::path::Path;
use store::{CommitStore, Error, KVStore, Result, TxId};

#[self_referencing]
pub struct RocksDBStore {
    db: TransactionDB,
    latest_tx_id: TxId,
    #[borrows(db)]
    #[covariant]
    txs: BTreeMap<TxId, StoreTransaction<'this>>,
}

#[self_referencing]
pub struct StoreTransaction<'a> {
    tx: Transaction<'a, TransactionDB>,
    #[borrows(tx)]
    #[covariant]
    snapshot: SnapshotWithThreadMode<'this, Transaction<'this, TransactionDB>>,
}

impl<'a> StoreTransaction<'a> {
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

impl<'a> KVStore for StoreTransaction<'a> {
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

unsafe impl Send for RocksDBStore {}
unsafe impl Sync for RocksDBStore {}

impl RocksDBStore {
    pub fn create(db: TransactionDB) -> Self {
        RocksDBStoreBuilder {
            db,
            latest_tx_id: Default::default(),
            txs_builder: |_| BTreeMap::default(),
        }
        .build()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        Self::create(TransactionDB::open_default(path).unwrap())
    }
}

impl CommitStore for RocksDBStore {
    fn begin(&mut self) -> Result<TxId> {
        self.with_mut(|fields| {
            // TODO make options configurable
            let mut tx_opt = TransactionOptions::default();
            tx_opt.set_snapshot(true);
            let tx = fields.db.transaction_opt(&WriteOptions::default(), &tx_opt);
            let transaction = StoreTransactionBuilder {
                tx,
                snapshot_builder: |tx| tx.snapshot(),
            }
            .build();
            fields.latest_tx_id.safe_incr()?;
            fields.txs.insert(*fields.latest_tx_id, transaction);
            Ok(*fields.latest_tx_id)
        })
    }

    fn commit(&mut self, tx_id: TxId) -> Result<()> {
        let tx = self.with_txs_mut(|txs| {
            let (_, tx) = txs.remove_entry(&tx_id).expect("tx not found");
            tx
        });
        tx.commit()
    }

    fn rollback(&mut self, tx_id: TxId) {
        let tx = self.with_txs_mut(|txs| {
            let (_, tx) = txs.remove_entry(&tx_id).expect("tx not found");
            tx
        });
        tx.rollback()
    }

    fn tx_get(&self, tx_id: TxId, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.with_txs(|txs| {
            Ok(txs
                .get(&tx_id)
                .ok_or_else(|| Error::tx_id_not_found(tx_id))?
                .get(key))
        })
    }

    fn tx_set(&mut self, tx_id: TxId, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.with_txs_mut(|txs| {
            txs.get_mut(&tx_id)
                .ok_or_else(|| Error::tx_id_not_found(tx_id))?
                .set(key, value);
            Ok(())
        })
    }

    fn tx_remove(&mut self, tx_id: TxId, key: &[u8]) -> Result<()> {
        self.with_txs_mut(|txs| {
            txs.get_mut(&tx_id)
                .ok_or_else(|| Error::tx_id_not_found(tx_id))?
                .remove(key);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_store() {
        let tmp_dir = TempDir::new("store-rocksdb").unwrap().into_path();
        let mut db = RocksDBStore::open(tmp_dir.clone());
        let tx_id = db.begin().unwrap();
        db.tx_set(tx_id, key(0), value(0)).unwrap();
        assert!(db.tx_get(tx_id, &key(0)).unwrap().eq(&Some(value(0))));
        db.commit(tx_id).unwrap();

        let tx_id = db.begin().unwrap();
        assert!(db.tx_get(tx_id, &key(0)).unwrap().eq(&Some(value(0))));
        db.commit(tx_id).unwrap();
    }

    fn key(idx: u32) -> Vec<u8> {
        format!("k{}", idx).into_bytes()
    }

    fn value(idx: u32) -> Vec<u8> {
        format!("v{}", idx).into_bytes()
    }
}

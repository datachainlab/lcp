use ouroboros::self_referencing;
use rocksdb::{Transaction, TransactionDB, TransactionOptions, WriteOptions};
use std::path::Path;
use store::{KVStore, Store, TransactionStore};

#[self_referencing]
pub struct RocksDBStore {
    db: TransactionDB,
    #[borrows(db)]
    #[covariant]
    tx: Option<Transaction<'this, TransactionDB>>,
}

unsafe impl Send for RocksDBStore {}
unsafe impl Sync for RocksDBStore {}

impl RocksDBStore {
    pub fn create(db: TransactionDB) -> Self {
        RocksDBStoreBuilder {
            db,
            tx_builder: |_| None,
        }
        .build()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        Self::create(TransactionDB::open_default(path).unwrap())
    }
}

impl Store for RocksDBStore {}

impl KVStore for RocksDBStore {
    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        self.with_tx(|tx| -> Option<Vec<u8>> {
            if let Some(tx) = tx {
                let v = tx.get(k).unwrap();
                v
            } else {
                panic!("current mode is not transaction")
            }
        })
    }
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.with_tx_mut(|tx| {
            if let Some(tx) = tx {
                tx.put(k, v).unwrap();
            } else {
                panic!("current mode is not transaction")
            }
        });
    }
}

impl TransactionStore for RocksDBStore {
    fn begin(&mut self) -> Result<(), store::Error> {
        self.with_mut(|fields| {
            if fields.tx.is_some() {
                panic!("transaction is already started")
            }
            *fields.tx = Some(
                fields
                    .db
                    .transaction_opt(&WriteOptions::default(), &TransactionOptions::default()),
            );
        });
        Ok(())
    }

    fn commit(&mut self) -> Result<(), store::Error> {
        let res =
            self.with_tx_mut(|tx| -> Result<(), rocksdb::Error> { tx.take().unwrap().commit() });
        let _ = res.unwrap();
        Ok(())
    }

    fn abort(&mut self) {
        self.with_tx_mut(|tx| {
            tx.take().unwrap().rollback().unwrap();
        });
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
        db.begin().unwrap();
        db.set(key(0), value(0));
        assert!(db.get(&key(0)).unwrap().eq(&value(0)));
        db.commit().unwrap();

        db.begin().unwrap();
        assert!(db.get(&key(0)).unwrap().eq(&value(0)));
        db.commit().unwrap();
    }

    fn key(idx: u32) -> Vec<u8> {
        format!("k{}", idx).into_bytes()
    }

    fn value(idx: u32) -> Vec<u8> {
        format!("v{}", idx).into_bytes()
    }
}

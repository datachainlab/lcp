use crate::prelude::*;
use crate::KVStore;
use alloc::collections::BTreeMap;
use core::cell::RefCell;

/// A key-value store that caches reads in memory
pub struct CacheKVS<S: KVStore> {
    parent: S,
    cache: RefCell<BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
}

impl<S: KVStore> CacheKVS<S> {
    pub fn new(parent: S) -> Self {
        Self {
            parent,
            cache: RefCell::new(BTreeMap::new()),
        }
    }
}

impl<S: KVStore> KVStore for CacheKVS<S> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.parent.set(key.clone(), value.clone());
        self.cache.borrow_mut().insert(key, Some(value));
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let cache = self.cache.borrow();
        let res = cache.get(key);
        match res {
            Some(Some(v)) => Some(v.clone()),
            Some(None) => None,
            None => {
                drop(cache);
                let v = self.parent.get(key);
                self.cache.borrow_mut().insert(key.to_vec(), v.clone());
                v
            }
        }
    }

    fn remove(&mut self, key: &[u8]) {
        self.parent.remove(key);
        self.cache.borrow_mut().insert(key.to_vec(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KVStore;
    use alloc::rc::Rc;

    pub struct MockStore {
        db: BTreeMap<Vec<u8>, Vec<u8>>,
    }

    impl MockStore {
        pub fn new() -> Self {
            Self {
                db: BTreeMap::new(),
            }
        }
    }

    impl KVStore for MockStore {
        fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
            self.db.insert(key, value);
        }

        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            self.db.get(key).cloned()
        }

        fn remove(&mut self, key: &[u8]) {
            self.db.remove(key);
        }
    }

    #[allow(non_snake_case)]
    fn B(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn test_cache_kvs() {
        let mut mock = Rc::new(RefCell::new(MockStore::new()));
        mock.set(B("k1"), B("v1"));

        let mut cache = CacheKVS::new(mock.clone());
        assert_eq!(cache.get(&B("k1")), Some(B("v1")));

        cache.set(B("k1"), B("v2"));
        assert_eq!(cache.get(&B("k1")), Some(B("v2")));
        assert_eq!(mock.get(&B("k1")), Some(B("v2")));

        mock.set(B("k1"), B("v3"));
        assert_eq!(mock.get(&B("k1")), Some(B("v3")));
        assert_eq!(cache.get(&B("k1")), Some(B("v2")));

        mock.remove(&B("k1"));
        assert_eq!(mock.get(&B("k1")), None);
        assert_eq!(cache.get(&B("k1")), Some(B("v2")));

        mock.set(B("k2"), B("v4"));
        assert_eq!(cache.get(&B("k2")), Some(B("v4")));

        assert_eq!(cache.get(&B("k3")), None);
        mock.set(B("k3"), B("v5"));
        assert_eq!(mock.get(&B("k3")), Some(B("v5")));
        assert_eq!(cache.get(&B("k3")), None);
        cache.set(B("k3"), B("v6"));
        assert_eq!(cache.get(&B("k3")), Some(B("v6")));

        cache.remove(&B("k4"));
        mock.set(B("k4"), B("v7"));
        assert_eq!(cache.get(&B("k4")), None);
        assert_eq!(mock.get(&B("k4")), Some(B("v7")));
        cache.set(B("k4"), B("v8"));
        assert_eq!(cache.get(&B("k4")), Some(B("v8")));
    }
}

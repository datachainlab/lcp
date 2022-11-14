use std::{
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use store::host::HostStore;

pub struct Environment {
    pub home: PathBuf,
    store: Arc<RwLock<HostStore>>,
}

impl Environment {
    pub fn new(home: PathBuf, store: Arc<RwLock<HostStore>>) -> Self {
        Self { home, store }
    }

    pub fn get_store(&self) -> RwLockReadGuard<HostStore> {
        self.store.read().unwrap()
    }

    pub fn get_mut_store(&self) -> RwLockWriteGuard<HostStore> {
        self.store.write().unwrap()
    }
}

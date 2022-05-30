use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::SgxRwLock;
use store::memory::MemStore;

// NOTE: use the mem store for debug
lazy_static! {
    pub static ref MEM_STORE: Arc<SgxRwLock<MemStore>> = Arc::new(SgxRwLock::new(MemStore::new()));
}

pub fn get_store() -> Arc<SgxRwLock<MemStore>> {
    return MEM_STORE.clone();
}

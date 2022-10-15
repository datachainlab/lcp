use lazy_static::lazy_static;
use sgx_tstd::sync::{Arc, SgxRwLock};
use store::memory::MemStore;

// NOTE: use the mem store for debug
lazy_static! {
    pub static ref MEM_STORE: Arc<SgxRwLock<MemStore>> =
        Arc::new(SgxRwLock::new(MemStore::default()));
}

pub fn get_store() -> Arc<SgxRwLock<MemStore>> {
    return MEM_STORE.clone();
}

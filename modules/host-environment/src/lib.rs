use std::sync::{Arc, RwLock};
use store::Store;

pub struct Environment {
    pub store: Arc<RwLock<dyn Store + Send + Sync>>,
}

impl Environment {
    pub fn new(store: Arc<RwLock<dyn Store + Send + Sync>>) -> Self {
        Self { store }
    }
}

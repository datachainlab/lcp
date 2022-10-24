use std::sync::{Arc, RwLock};
use store::CommitStore;

pub struct Environment {
    pub store: Arc<RwLock<dyn CommitStore + Send + Sync>>,
}

impl Environment {
    pub fn new(store: Arc<RwLock<dyn CommitStore + Send + Sync>>) -> Self {
        Self { store }
    }
}

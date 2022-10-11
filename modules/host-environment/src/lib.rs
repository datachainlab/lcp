pub struct Environment {
    // TODO env should keeps the store after refactoring host store
// pub store: Arc<RwLock<dyn KVStore + Send + Sync>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {}
    }
}

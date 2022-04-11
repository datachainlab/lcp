use enclave_crypto::KeyManager;
use lazy_static::lazy_static;
use std::sync::SgxRwLock;

lazy_static! {
    pub static ref KEY_MANAGER: SgxRwLock<KeyManager> = SgxRwLock::new(KeyManager::new());
}

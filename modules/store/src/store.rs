use crate::prelude::*;
use crate::{Error, Result};
use alloc::rc::Rc;
use core::cell::RefCell;
use core::fmt::Display;
use core::ops::Deref;
use serde::{Deserialize, Serialize};

pub trait KVStore {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn remove(&mut self, key: &[u8]);
}

impl KVStore for Box<dyn KVStore> {
    fn get(&self, k: &[u8]) -> Option<Vec<u8>> {
        self.as_ref().get(k)
    }
    fn set(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.as_mut().set(k, v)
    }
    fn remove(&mut self, key: &[u8]) {
        self.as_mut().remove(key)
    }
}

impl<T: KVStore> KVStore for Rc<RefCell<T>> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.borrow().get(key)
    }
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.borrow_mut().set(key, value)
    }
    fn remove(&mut self, key: &[u8]) {
        self.borrow_mut().remove(key)
    }
}

#[derive(
    Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct TxId(u64);

impl Deref for TxId {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for TxId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl TxId {
    pub fn safe_incr(&mut self) -> Result<()> {
        match self.0.checked_add(1) {
            Some(v) => {
                self.0 = v;
                Ok(())
            }
            None => Err(Error::tx_id_overflow()),
        }
    }
}

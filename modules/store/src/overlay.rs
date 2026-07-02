use crate::prelude::*;
use crate::{KVStore, WriteSet};

/// `OverlayKVS` is a speculative view over a parent key-value store.
///
/// Reads first consult the in-memory overlay and then fall back to the parent.
/// Writes are accumulated only in the overlay and never mutate the parent.
pub struct OverlayKVS<S: KVStore> {
    parent: S,
    overlay: WriteSet,
}

impl<S: KVStore> OverlayKVS<S> {
    pub fn new(parent: S) -> Self {
        Self {
            parent,
            overlay: WriteSet::default(),
        }
    }

    pub fn overlay(&self) -> &WriteSet {
        &self.overlay
    }

    pub fn into_parts(self) -> (S, WriteSet) {
        (self.parent, self.overlay)
    }
}

impl<S: KVStore> KVStore for OverlayKVS<S> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.overlay.insert(key, Some(value));
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self.overlay.get(key) {
            Some(Some(v)) => Some(v.clone()),
            Some(None) => None,
            None => self.parent.get(key),
        }
    }

    fn remove(&mut self, key: &[u8]) {
        self.overlay.insert(key.to_vec(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemStore;

    #[allow(non_snake_case)]
    fn B(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn overlay_reads_parent_when_missing() {
        let mut parent = MemStore::default();
        parent.set(B("k1"), B("v1"));

        let overlay = OverlayKVS::new(parent);
        assert_eq!(overlay.get(&B("k1")), Some(B("v1")));
        assert_eq!(overlay.get(&B("missing")), None);
    }

    #[test]
    fn overlay_write_shadows_parent_without_mutating_it() {
        let mut parent = MemStore::default();
        parent.set(B("k1"), B("v1"));

        let mut overlay = OverlayKVS::new(parent);
        overlay.set(B("k1"), B("v2"));
        overlay.set(B("k2"), B("v3"));

        assert_eq!(overlay.get(&B("k1")), Some(B("v2")));
        assert_eq!(overlay.get(&B("k2")), Some(B("v3")));

        let (parent, writes) = overlay.into_parts();
        assert_eq!(parent.get(&B("k1")), Some(B("v1")));
        assert_eq!(parent.get(&B("k2")), None);
        assert_eq!(writes.get(&B("k1")), Some(&Some(B("v2"))));
        assert_eq!(writes.get(&B("k2")), Some(&Some(B("v3"))));
    }

    #[test]
    fn overlay_delete_masks_parent_value() {
        let mut parent = MemStore::default();
        parent.set(B("k1"), B("v1"));

        let mut overlay = OverlayKVS::new(parent);
        overlay.remove(&B("k1"));

        assert_eq!(overlay.get(&B("k1")), None);

        let (parent, writes) = overlay.into_parts();
        assert_eq!(parent.get(&B("k1")), Some(B("v1")));
        assert_eq!(writes.get(&B("k1")), Some(&None));
    }
}

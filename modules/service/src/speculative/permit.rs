use enclave_api::Error as EnclaveError;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub(super) struct PermitGate {
    state: Mutex<PermitGateState>,
    ready: Condvar,
}

#[derive(Debug, Default)]
pub(super) struct KeyLockMap {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

#[derive(Debug)]
struct PermitGateState {
    available: usize,
}

struct PermitGuard<'a> {
    gate: &'a PermitGate,
}

impl PermitGate {
    pub(super) fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(PermitGateState {
                available: permits.max(1),
            }),
            ready: Condvar::new(),
        }
    }

    #[allow(clippy::result_large_err)]
    pub(super) fn with_permit<T>(
        &self,
        f: impl FnOnce() -> std::result::Result<T, EnclaveError>,
    ) -> std::result::Result<T, EnclaveError> {
        let _permit = self.acquire();
        f()
    }

    fn acquire(&self) -> PermitGuard<'_> {
        let mut state = self.state.lock().unwrap();
        while state.available == 0 {
            state = self.ready.wait(state).unwrap();
        }
        state.available -= 1;
        PermitGuard { gate: self }
    }
}

impl Drop for PermitGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.state.lock().unwrap();
        state.available += 1;
        self.gate.ready.notify_one();
    }
}

impl KeyLockMap {
    pub(super) fn with_key_serialized<T>(&self, key: &str, f: impl FnOnce() -> T) -> T {
        let lock = {
            let mut locks = self.locks.lock().unwrap();
            locks
                .entry(key.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let guard = lock.lock().unwrap();
        let result = f();
        drop(guard);

        let mut locks = self.locks.lock().unwrap();
        let should_remove = Arc::strong_count(&lock) == 2
            && locks
                .get(key)
                .map(|existing| Arc::ptr_eq(existing, &lock))
                .unwrap_or(false);
        if should_remove {
            locks.remove(key);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyLockMap, PermitGate};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn permit_gate_limits_concurrency() {
        let gate = Arc::new(PermitGate::new(2));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..6 {
            let gate = gate.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                gate.with_permit(|| {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    observed_max.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(25));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(observed_max.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn key_lock_map_serializes_same_key() {
        let locks = Arc::new(KeyLockMap::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..6 {
            let locks = locks.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                locks.with_key_serialized("client-0", || {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    observed_max.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(25));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(observed_max.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn key_lock_map_allows_different_keys() {
        let locks = Arc::new(KeyLockMap::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for i in 0..6 {
            let locks = locks.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                locks.with_key_serialized(&format!("client-{i}"), || {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    observed_max.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(25));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(observed_max.load(Ordering::SeqCst) > 1);
    }
}

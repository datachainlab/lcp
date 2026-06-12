use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub(crate) struct ClientUpdateLocks {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ClientUpdateLocks {
    pub(crate) fn with_client_serialized<T>(&self, client_id: &str, f: impl FnOnce() -> T) -> T {
        let lock = {
            let mut locks = self.locks.lock().unwrap();
            locks
                .entry(client_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        // Recover from poisoning: the guarded data is `()`, so a panic while
        // the lock was held leaves no invariant to protect, and refusing the
        // lock here would permanently block all updates for this client.
        let guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = f();
        drop(guard);

        let mut locks = self.locks.lock().unwrap();
        // strong_count == 2 means only this local `lock` binding and the map
        // entry still reference the mutex, so the idle key entry can be removed.
        let should_remove = Arc::strong_count(&lock) == 2
            && locks
                .get(client_id)
                .map(|existing| Arc::ptr_eq(existing, &lock))
                .unwrap_or(false);
        if should_remove {
            locks.remove(client_id);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::ClientUpdateLocks;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn client_update_locks_serialize_same_client() {
        let locks = Arc::new(ClientUpdateLocks::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..6 {
            let locks = locks.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                locks.with_client_serialized("client-0", || {
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
    fn client_update_locks_recover_after_panicked_update() {
        let locks = ClientUpdateLocks::default();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            locks.with_client_serialized("client-0", || {
                panic!("injected update panic");
            })
        }));
        assert!(panic.is_err());
        // A panicked update must not poison the per-client lock for good.
        let value = locks.with_client_serialized("client-0", || 42);
        assert_eq!(value, 42);
    }

    #[test]
    fn client_update_locks_allow_different_clients() {
        let locks = Arc::new(ClientUpdateLocks::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for i in 0..6 {
            let locks = locks.clone();
            let in_flight = in_flight.clone();
            let observed_max = observed_max.clone();
            handles.push(thread::spawn(move || {
                locks.with_client_serialized(&format!("client-{i}"), || {
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

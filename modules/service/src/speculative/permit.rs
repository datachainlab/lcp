use enclave_api::Error as EnclaveError;
use std::sync::{Condvar, Mutex};

#[derive(Debug)]
pub(super) struct PermitGate {
    state: Mutex<PermitGateState>,
    ready: Condvar,
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

#[cfg(test)]
mod tests {
    use super::PermitGate;
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
}

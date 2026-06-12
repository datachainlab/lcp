use log::*;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A fixed-size pool of long-lived OS threads dedicated to executing ECALLs.
///
/// Under `TCSPolicy=BIND`, the Intel SGX SDK binds a TCS to each host thread
/// on its first ECALL and only releases the binding when the thread
/// terminates. Without an upper bound on the set of distinct threads that
/// ever ECALL, cumulative bindings can exceed `TCSNum` even when concurrent
/// ECALLs stay well below it, producing transient `SGX_ERROR_OUT_OF_TCS`
/// failures.
///
/// `EcallPool` solves this by pinning ECALL execution to exactly `size`
/// permanent worker threads. Workers are spawned once at service start and
/// live for the entire process lifetime; their TCS bindings are therefore
/// stable at `size` and never accumulate.
pub struct EcallPool {
    sender: Option<Sender<Job>>,
    workers: Vec<JoinHandle<()>>,
}

impl EcallPool {
    /// Creates a pool with `size` permanent worker threads (`size.max(1)`).
    /// Callers should set `size` equal to `--max-enclave-concurrency`.
    pub fn new(size: usize) -> Self {
        let size = size.max(1);
        let (sender, receiver) = channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..size)
            .map(|i| {
                let receiver = Arc::clone(&receiver);
                thread::Builder::new()
                    .name(format!("ecall-{}", i))
                    .spawn(move || ecall_worker_loop(i, receiver))
                    .expect("failed to spawn ECALL pool worker")
            })
            .collect();
        Self {
            sender: Some(sender),
            workers,
        }
    }

    /// Runs `f` on one of the pool's worker threads, blocking the caller
    /// until the job completes. Each invocation acquires a worker slot.
    pub fn run<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let sender = self
            .sender
            .as_ref()
            .expect("ECALL pool used after shutdown");
        let (tx, rx) = channel();
        let job: Job = Box::new(move || {
            let _ = tx.send(f());
        });
        sender.send(job).expect("ECALL pool worker channel closed");
        rx.recv()
            .expect("ECALL pool worker terminated before producing a result")
    }
}

impl Drop for EcallPool {
    fn drop(&mut self) {
        // Closing the sender lets each worker observe `Err` on `recv` and
        // exit its loop. We then join every worker so SGX SDK destructors
        // run before the surrounding process resources are torn down.
        drop(self.sender.take());
        for worker in self.workers.drain(..) {
            if let Err(e) = worker.join() {
                warn!("ECALL pool worker panicked at shutdown: {:?}", e);
            }
        }
    }
}

fn ecall_worker_loop(index: usize, receiver: Arc<Mutex<std::sync::mpsc::Receiver<Job>>>) {
    debug!("ECALL worker {} started", index);
    loop {
        let job = {
            let recv = receiver.lock().unwrap();
            recv.recv()
        };
        match job {
            Ok(job) => job(),
            Err(_) => {
                debug!("ECALL worker {} exiting (channel closed)", index);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EcallPool;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn pool_limits_concurrent_jobs_to_worker_count() {
        let pool = Arc::new(EcallPool::new(2));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let observed_max = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..6 {
            let pool = Arc::clone(&pool);
            let in_flight = Arc::clone(&in_flight);
            let observed_max = Arc::clone(&observed_max);
            handles.push(thread::spawn(move || {
                pool.run(move || {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    observed_max.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(40));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(observed_max.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn pool_returns_job_result_to_caller() {
        let pool = EcallPool::new(2);
        let result = pool.run(|| 7 * 6);
        assert_eq!(result, 42);
    }

    #[test]
    fn pool_workers_have_stable_thread_ids_across_jobs() {
        // Verifies the "1 thread = 1 TCS forever" property under BIND policy:
        // the set of OS thread ids that execute jobs is bounded by pool size.
        let pool = Arc::new(EcallPool::new(3));
        let observed = Arc::new(std::sync::Mutex::new(std::collections::HashSet::<
            thread::ThreadId,
        >::new()));
        let mut handles = Vec::new();
        for _ in 0..30 {
            let pool = Arc::clone(&pool);
            let observed = Arc::clone(&observed);
            handles.push(thread::spawn(move || {
                pool.run(move || {
                    observed.lock().unwrap().insert(thread::current().id());
                    thread::sleep(Duration::from_millis(5));
                });
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let set = observed.lock().unwrap();
        assert!(
            set.len() <= 3,
            "expected at most pool-size distinct worker threads, saw {}",
            set.len()
        );
    }
}

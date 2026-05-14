use super::permit::{KeyLockMap, PermitGate};
#[cfg(test)]
use super::rebase::{
    extract_client_state_from_write_set, extract_consensus_state_from_write_set,
    rebase_speculative_request_in_place, DependencyRebaseState,
};
use super::scheduler::execute_speculative_update_client_stream;
use super::stream::ResidentSpeculativeUpdateClientRequest;
use super::types::{
    ExplicitStateRef, ObservedStateTransition, SpeculativeBatchFailure,
    SpeculativeBatchFailureKind, SpeculativeUpdateClientBatch, SpeculativeUpdateClientBatchResult,
    SpeculativeUpdateClientRequest, SpeculativeUpdateClientResult, StitchedUpdateClientBatchResult,
    StitchedUpdateClientResult,
};
use super::validation::{validate_linear_batch_requests, validate_linear_transitions};
use crate::service::AppService;
use commitments::ProxyMessage;
use enclave_api::{
    EnclaveProtoAPI, Error as EnclaveError, SpeculativeBaseState, SpeculativeEnclaveCommandAPI,
    SpeculativeUpdateClientInput as EnclaveSpeculativeUpdateClientInput,
};
#[cfg(test)]
use lcp_proto::lcp::service::elc::v1::{MsgUpdateClient, MsgUpdateClientResponse};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use store::transaction::{CommitStore, TxAccessor};
use store::WriteSet;

pub struct SpeculativeService {
    key_locks: Arc<KeyLockMap>,
    speculative_concurrency_limit: usize,
    speculative_request_permits: Arc<PermitGate>,
}

impl Clone for SpeculativeService {
    fn clone(&self) -> Self {
        Self {
            key_locks: self.key_locks.clone(),
            speculative_concurrency_limit: self.speculative_concurrency_limit,
            speculative_request_permits: self.speculative_request_permits.clone(),
        }
    }
}

impl SpeculativeService {
    pub fn new(speculative_concurrency_limit: usize) -> Self {
        Self {
            key_locks: Arc::new(KeyLockMap::default()),
            speculative_concurrency_limit: speculative_concurrency_limit.max(1),
            speculative_request_permits: Arc::new(PermitGate::new(speculative_concurrency_limit)),
        }
    }

    pub fn speculative_concurrency_limit(&self) -> usize {
        self.speculative_concurrency_limit
    }

    pub fn with_client_serialized<T>(&self, client_id: &str, f: impl FnOnce() -> T) -> T {
        // Keep client-key serialization outside the speculative execution/stitch
        // body so all canonical writes for one client are ordered.
        self.key_locks.with_key_serialized(client_id, f)
    }

    #[allow(clippy::result_large_err)]
    pub fn with_speculative_request_permit<T>(
        &self,
        f: impl FnOnce() -> std::result::Result<T, EnclaveError>,
    ) -> std::result::Result<T, EnclaveError> {
        self.speculative_request_permits.with_permit(f)
    }

    #[allow(clippy::result_large_err)]
    pub fn speculative_update_client<E, S>(
        &self,
        app: &AppService<E, S>,
        req: SpeculativeUpdateClientRequest,
    ) -> core::result::Result<SpeculativeUpdateClientResult, enclave_api::Error>
    where
        S: CommitStore + TxAccessor + 'static,
        E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + 'static,
    {
        let update = req.update.try_into()?;
        let base_state = req.base_state.clone();
        let res = app
            .enclave
            .speculative_update_client(EnclaveSpeculativeUpdateClientInput {
                update,
                base_state: base_state_payload_from_ref(&base_state),
            })?;
        let observed_transition = decode_observed_transition(&res.response)?;
        Ok(SpeculativeUpdateClientResult {
            response: res.response.into(),
            write_set: res.write_set,
            base_state: req.base_state,
            observed_transition,
        })
    }

    pub fn stitch_speculative_update_client_batch<E, S>(
        &self,
        app: &AppService<E, S>,
        batch: SpeculativeUpdateClientBatch,
        results: SpeculativeUpdateClientBatchResult,
    ) -> core::result::Result<StitchedUpdateClientBatchResult, SpeculativeBatchFailure>
    where
        S: CommitStore + TxAccessor + 'static,
        E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + 'static,
    {
        validate_linear_batch_requests(&batch.client_id, &batch.units)?;
        if batch.client_id != results.client_id {
            return Err(SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::ResultClientMismatch,
                unit_id: None,
                detail: format!(
                    "batch result client_id mismatch: expected={} observed={}",
                    batch.client_id, results.client_id
                ),
            });
        }
        if batch.units.len() != results.units.len() {
            return Err(SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
                unit_id: None,
                detail: format!(
                    "batch size mismatch: requests={} results={}",
                    batch.units.len(),
                    results.units.len()
                ),
            });
        }
        validate_linear_transitions(&batch.units, &results.units)?;

        let mut merged_write_set = WriteSet::default();
        let mut units = Vec::with_capacity(batch.units.len());
        for (req, result) in batch.units.iter().zip(results.units.into_iter()) {
            result
                .validate_base_state()
                .map_err(|e| SpeculativeBatchFailure {
                    kind: SpeculativeBatchFailureKind::BaseStateMismatch,
                    unit_id: Some(req.unit_id.clone()),
                    detail: e.to_string(),
                })?;
            for (key, value) in result.write_set {
                merged_write_set.insert(key, value);
            }
            units.push(StitchedUpdateClientResult {
                response: result.response,
                observed_transition: result.observed_transition,
            });
        }
        app.enclave
            .apply_write_set(batch.client_id.clone(), merged_write_set)
            .map_err(|e| SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::StitchApplyFailed,
                unit_id: None,
                detail: e.to_string(),
            })?;

        Ok(StitchedUpdateClientBatchResult {
            client_id: batch.client_id,
            units,
        })
    }

    pub(crate) fn execute_serialized_speculative_update_client_stream<E, S>(
        &self,
        app: &AppService<E, S>,
        client_id: String,
        units: Receiver<ResidentSpeculativeUpdateClientRequest>,
    ) -> core::result::Result<StitchedUpdateClientBatchResult, SpeculativeBatchFailure>
    where
        S: CommitStore + TxAccessor + Send + 'static,
        E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + Send + Sync + 'static,
    {
        self.with_client_serialized(&client_id.clone(), || {
            let batch_result =
                execute_speculative_update_client_stream(self, app, client_id.clone(), units)?;
            let batch = SpeculativeUpdateClientBatch {
                client_id,
                units: batch_result.requests,
            };
            self.stitch_speculative_update_client_batch(app, batch, batch_result.results)
        })
    }
}

fn base_state_payload_from_ref(base_state: &ExplicitStateRef) -> Option<SpeculativeBaseState> {
    Some(SpeculativeBaseState {
        prev_height: Some(base_state.prev_height?),
        client_state: base_state.client_state.clone()?,
        consensus_state: base_state.consensus_state.clone()?,
    })
}

#[allow(clippy::result_large_err)]
fn decode_observed_transition(
    response: &ecall_commands::UpdateClientResponse,
) -> core::result::Result<ObservedStateTransition, enclave_api::Error> {
    match response.0.message()? {
        ProxyMessage::UpdateState(msg) => Ok(ObservedStateTransition {
            prev_height: msg.prev_height,
            prev_state_id: msg.prev_state_id.map(|id| id.to_vec()),
            post_height: msg.post_height,
            post_state_id: msg.post_state_id.to_vec(),
        }),
        other => Err(enclave_api::Error::invalid_argument(format!(
            "expected UpdateState proxy message, got {:?}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commitments::{CommitmentProof, StateID, UpdateStateProxyMessage, ValidationContext};
    use ecall_commands::UpdateClientResponse as EnclaveUpdateClientResponse;
    use enclave_api::{
        CommitStoreAccessor, EnclaveCommandAPI, EnclaveInfo, EnclavePrimitiveAPI, EnclaveProtoAPI,
        HostStoreTxManager, SpeculativeEnclaveCommandAPI,
        SpeculativeUpdateClientInput as EnclaveSpeculativeUpdateClientInput,
        SpeculativeUpdateClientResponse as EnclaveSpeculativeUpdateClientResponse,
    };
    use keymanager::EnclaveKeyManager;
    use lcp_proto::google::protobuf::Any;
    use lcp_types::{store_key, Height};
    use lcp_types::{EnclaveMetadata, Time};
    use sgx_types::{sgx_enclave_id_t, sgx_status_t};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;
    use store::memory::MemStore;

    struct FakeEnclave {
        store: Mutex<MemStore>,
        key_manager: EnclaveKeyManager,
        current_in_flight: AtomicUsize,
        observed_max_in_flight: AtomicUsize,
        delay: Duration,
    }

    impl FakeEnclave {
        fn new(delay: Duration) -> Self {
            let key_manager_home = std::env::temp_dir().join(format!(
                "lcp-fake-enclave-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time")
                    .as_nanos()
            ));
            std::fs::create_dir_all(&key_manager_home).expect("fake enclave key manager home");
            Self {
                store: Mutex::new(MemStore::default()),
                key_manager: EnclaveKeyManager::new(&key_manager_home)
                    .expect("fake enclave key manager"),
                current_in_flight: AtomicUsize::new(0),
                observed_max_in_flight: AtomicUsize::new(0),
                delay,
            }
        }

        fn observed_max_in_flight(&self) -> usize {
            self.observed_max_in_flight.load(Ordering::SeqCst)
        }
    }

    impl CommitStoreAccessor<MemStore> for FakeEnclave {
        fn use_mut_store<T>(&self, f: impl FnOnce(&mut MemStore) -> T) -> T {
            let mut store = self.store.lock().unwrap();
            f(&mut store)
        }
    }

    impl HostStoreTxManager<MemStore> for FakeEnclave {}

    impl EnclaveInfo for FakeEnclave {
        fn get_eid(&self) -> sgx_enclave_id_t {
            0
        }

        fn metadata(&self) -> core::result::Result<EnclaveMetadata, sgx_status_t> {
            unimplemented!("metadata is not used in explicit-state unit tests")
        }

        fn is_debug(&self) -> bool {
            false
        }

        fn get_key_manager(&self) -> &EnclaveKeyManager {
            &self.key_manager
        }
    }

    impl EnclavePrimitiveAPI<MemStore> for FakeEnclave {}

    impl EnclaveCommandAPI<MemStore> for FakeEnclave {}

    impl SpeculativeEnclaveCommandAPI<MemStore> for FakeEnclave {
        fn speculative_update_client(
            &self,
            input: EnclaveSpeculativeUpdateClientInput,
        ) -> core::result::Result<EnclaveSpeculativeUpdateClientResponse, enclave_api::Error>
        {
            let idx = input.update.signer.0[19] as u64;
            let current = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.observed_max_in_flight
                .fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(self.delay);
            self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

            let prev_height = (idx > 0).then(|| Height::new(0, 10 + idx));
            let prev_state_id = (idx > 0).then(|| {
                let mut prev_state_id = [0u8; 32];
                prev_state_id[31] = idx as u8;
                StateID::from(prev_state_id)
            });
            let mut post_state_id = [0u8; 32];
            post_state_id[31] = (idx as u8) + 1;
            let message = ProxyMessage::from(UpdateStateProxyMessage {
                prev_height,
                prev_state_id,
                post_height: Height::new(0, 10 + idx + 1),
                post_state_id: StateID::from(post_state_id),
                timestamp: Time::unix_epoch(),
                context: ValidationContext::Empty,
                emitted_states: vec![],
            })
            .to_bytes();

            Ok(EnclaveSpeculativeUpdateClientResponse {
                response: EnclaveUpdateClientResponse(CommitmentProof::new_with_no_signature(
                    message,
                )),
                write_set: vec![(vec![idx as u8], Some(vec![idx as u8]))]
                    .into_iter()
                    .collect(),
            })
        }
    }

    impl EnclaveProtoAPI<MemStore> for FakeEnclave {}

    fn mk_req(
        unit_id: &str,
        client_id: &str,
        prev_height: Option<Height>,
        prev_state_id: Option<&[u8]>,
    ) -> SpeculativeUpdateClientRequest {
        SpeculativeUpdateClientRequest {
            unit_id: unit_id.to_string(),
            update: MsgUpdateClient {
                client_id: client_id.to_string(),
                header: Some(Any {
                    type_url: "/ibc.mock.Header".to_string(),
                    value: vec![1],
                }),
                ..Default::default()
            },
            base_state: ExplicitStateRef {
                prev_height,
                prev_state_id: prev_state_id.map(|v| v.to_vec()),
                client_state: None,
                consensus_state: None,
            },
        }
    }

    fn with_explicit_base_state_payload(
        mut req: SpeculativeUpdateClientRequest,
    ) -> SpeculativeUpdateClientRequest {
        req.base_state.client_state = Some(
            Any {
                type_url: "/ibc.mock.ClientState".to_string(),
                value: vec![1],
            }
            .into(),
        );
        req.base_state.consensus_state = Some(
            Any {
                type_url: "/ibc.mock.ConsensusState".to_string(),
                value: vec![2],
            }
            .into(),
        );
        req
    }

    fn mk_result(
        prev_height: Option<Height>,
        prev_state_id: Option<&[u8]>,
        post_height: Height,
        post_state_id: &[u8],
    ) -> SpeculativeUpdateClientResult {
        SpeculativeUpdateClientResult {
            response: MsgUpdateClientResponse::default(),
            write_set: WriteSet::default(),
            base_state: ExplicitStateRef {
                prev_height,
                prev_state_id: prev_state_id.map(|v| v.to_vec()),
                client_state: None,
                consensus_state: None,
            },
            observed_transition: ObservedStateTransition {
                prev_height,
                prev_state_id: prev_state_id.map(|v| v.to_vec()),
                post_height,
                post_state_id: post_state_id.to_vec(),
            },
        }
    }

    #[test]
    fn validates_linear_state_transitions() {
        let requests = vec![
            mk_req("unit-0000", "client", None, None),
            mk_req(
                "unit-0001",
                "client",
                Some(Height::new(0, 11)),
                Some(b"post-0"),
            ),
        ];
        let results = vec![
            mk_result(None, None, Height::new(0, 11), b"post-0"),
            mk_result(
                Some(Height::new(0, 11)),
                Some(b"post-0"),
                Height::new(0, 12),
                b"post-1",
            ),
        ];

        assert!(validate_linear_transitions(&requests, &results).is_ok());
    }

    #[test]
    fn validates_base_state_prev_height_only_when_provided() {
        let mut result = mk_result(
            Some(Height::new(0, 11)),
            None,
            Height::new(0, 12),
            b"post-1",
        );
        result.base_state.prev_height = None;

        result
            .validate_base_state()
            .expect("missing prev_height should accept observed height");
    }

    #[test]
    fn rejects_base_state_prev_height_mismatch_when_provided() {
        let mut result = mk_result(
            Some(Height::new(0, 11)),
            None,
            Height::new(0, 12),
            b"post-1",
        );
        result.base_state.prev_height = Some(Height::new(0, 10));

        let err = result.validate_base_state().unwrap_err();
        assert!(err.to_string().contains("base prev_height mismatch"));
    }

    #[test]
    fn rejects_linear_state_mismatch() {
        let requests = vec![
            mk_req("unit-0000", "client", None, None),
            mk_req(
                "unit-0001",
                "client",
                Some(Height::new(0, 11)),
                Some(b"wrong"),
            ),
        ];
        let results = vec![
            mk_result(None, None, Height::new(0, 11), b"post-0"),
            mk_result(
                Some(Height::new(0, 11)),
                Some(b"wrong"),
                Height::new(0, 12),
                b"post-1",
            ),
        ];

        let err = validate_linear_transitions(&requests, &results).unwrap_err();
        assert_eq!(
            err.kind,
            SpeculativeBatchFailureKind::DependencyStateMismatch
        );
        assert_eq!(err.unit_id.as_deref(), Some("unit-0001"));
    }

    #[test]
    fn replaces_explicit_base_state_metadata_when_rebasing_previous_payloads() {
        let mut req = mk_req(
            "unit-0001",
            "client",
            Some(Height::new(0, 10)),
            Some(b"stale"),
        );
        let previous = DependencyRebaseState {
            observed_transition: ObservedStateTransition {
                prev_height: None,
                prev_state_id: None,
                post_height: Height::new(0, 11),
                post_state_id: b"post-0".to_vec(),
            },
            client_state: None,
            consensus_state: None,
        };

        rebase_speculative_request_in_place(&mut req, &previous);

        assert_eq!(req.base_state.prev_height, Some(Height::new(0, 11)));
        assert_eq!(
            req.base_state.prev_state_id.as_deref(),
            Some(b"post-0".as_slice())
        );
    }

    #[test]
    fn fills_missing_base_state_metadata_from_previous_post_state() {
        let mut req = mk_req("unit-0001", "client", None, None);
        let previous = DependencyRebaseState {
            observed_transition: ObservedStateTransition {
                prev_height: None,
                prev_state_id: None,
                post_height: Height::new(0, 11),
                post_state_id: b"post-0".to_vec(),
            },
            client_state: None,
            consensus_state: None,
        };

        rebase_speculative_request_in_place(&mut req, &previous);

        assert_eq!(req.base_state.prev_height, Some(Height::new(0, 11)));
        assert_eq!(
            req.base_state.prev_state_id.as_deref(),
            Some(b"post-0".as_slice())
        );
    }

    #[test]
    fn seeds_previous_payloads_even_when_explicit_base_state_is_complete() {
        let mut req = with_explicit_base_state_payload(mk_req(
            "unit-0001",
            "client",
            Some(Height::new(0, 11)),
            Some(b"post-0"),
        ));
        let previous = DependencyRebaseState {
            observed_transition: ObservedStateTransition {
                prev_height: None,
                prev_state_id: None,
                post_height: Height::new(0, 11),
                post_state_id: b"post-0".to_vec(),
            },
            client_state: Some(
                Any {
                    type_url: "/ibc.mock.ClientState".to_string(),
                    value: vec![3],
                }
                .into(),
            ),
            consensus_state: Some(
                Any {
                    type_url: "/ibc.mock.ConsensusState".to_string(),
                    value: vec![4],
                }
                .into(),
            ),
        };

        rebase_speculative_request_in_place(&mut req, &previous);

        assert_eq!(req.base_state.prev_height, Some(Height::new(0, 11)));
        assert_eq!(
            req.base_state.prev_state_id.as_deref(),
            Some(b"post-0".as_slice())
        );
        assert!(req.base_state.client_state.is_some());
        assert!(req.base_state.consensus_state.is_some());
    }

    #[test]
    fn extracts_rebase_payloads_from_bincode_write_set() {
        let client_id = "07-tendermint-0";
        let height = Height::new(0, 11);
        let client_state = Any {
            type_url: "/ibc.mock.ClientState".to_string(),
            value: vec![1, 2, 3],
        };
        let consensus_state = Any {
            type_url: "/ibc.mock.ConsensusState".to_string(),
            value: vec![4, 5, 6],
        };
        let client_state_key = store_key::client_state_bytes(client_id);
        let consensus_state_key = store_key::consensus_state_bytes(client_id, &height);
        let mut write_set = WriteSet::default();
        write_set.insert(
            client_state_key,
            Some(
                bincode::serde::encode_to_vec(&client_state, bincode::config::standard())
                    .expect("encode client state"),
            ),
        );
        write_set.insert(
            consensus_state_key,
            Some(
                bincode::serde::encode_to_vec(&consensus_state, bincode::config::standard())
                    .expect("encode consensus state"),
            ),
        );

        assert_eq!(
            extract_client_state_from_write_set(client_id, &write_set),
            Some(client_state.into())
        );
        assert_eq!(
            extract_consensus_state_from_write_set(client_id, height, &write_set),
            Some(consensus_state.into())
        );
    }

    #[test]
    fn ignores_missing_or_malformed_rebase_payloads_from_write_set() {
        let client_id = "07-tendermint-0";
        let height = Height::new(0, 11);
        let client_state_key = store_key::client_state_bytes(client_id);
        let consensus_state_key = store_key::consensus_state_bytes(client_id, &height);
        let mut write_set = WriteSet::default();
        write_set.insert(client_state_key, Some(b"not-bincode-any".to_vec()));
        write_set.insert(consensus_state_key, None);

        assert_eq!(
            extract_client_state_from_write_set(client_id, &write_set),
            None
        );
        assert_eq!(
            extract_consensus_state_from_write_set(client_id, height, &write_set),
            None
        );
    }

    #[test]
    fn streaming_speculative_batch_executes_before_input_closes() {
        let client_id = "07-tendermint-0";
        let enclave = FakeEnclave::new(Duration::from_millis(100));
        let app = AppService::<FakeEnclave, MemStore>::new("test-home", enclave);
        let service = SpeculativeService::new(2);
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let worker_service = service.clone();
        let worker_app = app.clone();
        let client_id_for_worker = client_id.to_string();
        let handle = thread::spawn(move || {
            worker_service.execute_serialized_speculative_update_client_stream(
                &worker_app,
                client_id_for_worker,
                rx,
            )
        });

        tx.send(ResidentSpeculativeUpdateClientRequest::unmetered(
            SpeculativeUpdateClientRequest {
                unit_id: "unit-0000".to_string(),
                update: MsgUpdateClient {
                    client_id: client_id.to_string(),
                    signer: vec![0; 20],
                    header: Some(Any {
                        type_url: "/ibc.mock.Header".to_string(),
                        value: vec![1],
                    }),
                    ..Default::default()
                },
                base_state: ExplicitStateRef {
                    prev_height: None,
                    prev_state_id: None,
                    client_state: None,
                    consensus_state: None,
                },
            },
        ))
        .expect("send first unit");

        for _ in 0..100 {
            if app.enclave.observed_max_in_flight() >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            app.enclave.observed_max_in_flight() >= 1,
            "expected first unit to start before input stream closes"
        );

        tx.send(ResidentSpeculativeUpdateClientRequest::unmetered(
            SpeculativeUpdateClientRequest {
                unit_id: "unit-0001".to_string(),
                update: MsgUpdateClient {
                    client_id: client_id.to_string(),
                    signer: {
                        let mut signer = vec![0; 20];
                        signer[19] = 1;
                        signer
                    },
                    header: Some(Any {
                        type_url: "/ibc.mock.Header".to_string(),
                        value: vec![2],
                    }),
                    ..Default::default()
                },
                base_state: ExplicitStateRef {
                    prev_height: None,
                    prev_state_id: None,
                    client_state: None,
                    consensus_state: None,
                },
            },
        ))
        .expect("send second unit");
        drop(tx);

        let result = handle
            .join()
            .expect("streaming worker thread")
            .expect("streaming speculative batch");
        assert_eq!(result.units.len(), 2);
        assert_eq!(app.enclave.observed_max_in_flight(), 1);
    }

    #[test]
    fn streaming_speculative_batch_parallelizes_complete_base_state_units() {
        let client_id = "07-tendermint-0";
        let enclave = FakeEnclave::new(Duration::from_millis(100));
        let app = AppService::<FakeEnclave, MemStore>::new("test-home", enclave);
        let service = SpeculativeService::new(3);
        let (tx, rx) = std::sync::mpsc::sync_channel(3);
        let worker_service = service.clone();
        let worker_app = app.clone();
        let client_id_for_worker = client_id.to_string();
        let handle = thread::spawn(move || {
            worker_service.execute_serialized_speculative_update_client_stream(
                &worker_app,
                client_id_for_worker,
                rx,
            )
        });

        let mut requests = vec![
            with_explicit_base_state_payload(mk_req("unit-0000", client_id, None, None)),
            with_explicit_base_state_payload(mk_req(
                "unit-0001",
                client_id,
                Some(Height::new(0, 11)),
                None,
            )),
            with_explicit_base_state_payload(mk_req(
                "unit-0002",
                client_id,
                Some(Height::new(0, 12)),
                None,
            )),
        ];
        for (i, req) in requests.iter_mut().enumerate() {
            req.update.signer = {
                let mut signer = vec![0; 20];
                signer[19] = i as u8;
                signer
            };
        }
        for req in requests {
            tx.send(ResidentSpeculativeUpdateClientRequest::unmetered(req))
                .expect("send unit");
        }
        drop(tx);

        let result = handle
            .join()
            .expect("streaming worker thread")
            .expect("streaming speculative batch");
        assert_eq!(result.units.len(), 3);
        assert!(
            app.enclave.observed_max_in_flight() >= 2,
            "expected complete base-state units to run concurrently, saw {}",
            app.enclave.observed_max_in_flight()
        );
    }
}

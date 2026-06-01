use super::permit::PermitGate;
use super::scheduler::execute_speculative_update_client_stream as execute_stream_scheduler;
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
    speculative_concurrency_limit: usize,
    speculative_request_permits: Arc<PermitGate>,
}

impl Clone for SpeculativeService {
    fn clone(&self) -> Self {
        Self {
            speculative_concurrency_limit: self.speculative_concurrency_limit,
            speculative_request_permits: self.speculative_request_permits.clone(),
        }
    }
}

impl SpeculativeService {
    pub fn new(speculative_concurrency_limit: usize) -> Self {
        Self {
            speculative_concurrency_limit: speculative_concurrency_limit.max(1),
            speculative_request_permits: Arc::new(PermitGate::new(speculative_concurrency_limit)),
        }
    }

    pub fn speculative_concurrency_limit(&self) -> usize {
        self.speculative_concurrency_limit
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
                base_state: base_state_payload_from_ref(&base_state)?,
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
        let first_unit = batch.units.first().ok_or_else(|| SpeculativeBatchFailure {
            kind: SpeculativeBatchFailureKind::BatchSizeMismatch,
            unit_id: None,
            detail: "speculative batch must contain at least one unit".to_string(),
        })?;
        let canonical_base = base_state_payload_from_ref(&first_unit.base_state).map_err(|e| {
            SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::BaseStateMismatch,
                unit_id: Some(first_unit.unit_id.clone()),
                detail: e.to_string(),
            }
        })?;

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
            .apply_write_set_with_expected_base(
                batch.client_id.clone(),
                canonical_base.prev_height,
                &canonical_base.client_state,
                &canonical_base.consensus_state,
                merged_write_set,
            )
            .map_err(|e| SpeculativeBatchFailure {
                kind: SpeculativeBatchFailureKind::BaseStateMismatch,
                unit_id: Some(first_unit.unit_id.clone()),
                detail: e.to_string(),
            })?;

        Ok(StitchedUpdateClientBatchResult {
            client_id: batch.client_id,
            units,
        })
    }

    pub(crate) fn execute_speculative_update_client_stream<E, S>(
        &self,
        app: &AppService<E, S>,
        client_id: String,
        units: Receiver<ResidentSpeculativeUpdateClientRequest>,
    ) -> core::result::Result<StitchedUpdateClientBatchResult, SpeculativeBatchFailure>
    where
        S: CommitStore + TxAccessor + Send + 'static,
        E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + Send + Sync + 'static,
    {
        let batch_result = execute_stream_scheduler(self, app, client_id.clone(), units)?;
        let batch = SpeculativeUpdateClientBatch {
            client_id,
            units: batch_result.requests,
        };
        self.stitch_speculative_update_client_batch(app, batch, batch_result.results)
    }
}

#[allow(clippy::result_large_err)]
fn base_state_payload_from_ref(
    base_state: &ExplicitStateRef,
) -> core::result::Result<SpeculativeBaseState, enclave_api::Error> {
    let prev_height = base_state.prev_height.ok_or_else(|| {
        enclave_api::Error::invalid_argument(
            "speculative base_state prev_height must be provided".to_string(),
        )
    })?;
    let client_state = base_state.client_state.clone().ok_or_else(|| {
        enclave_api::Error::invalid_argument(
            "speculative base_state client_state must be provided".to_string(),
        )
    })?;
    let consensus_state = base_state.consensus_state.clone().ok_or_else(|| {
        enclave_api::Error::invalid_argument(
            "speculative base_state consensus_state must be provided".to_string(),
        )
    })?;

    Ok(SpeculativeBaseState {
        prev_height,
        client_state,
        consensus_state,
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
    use lcp_types::Height;
    use lcp_types::{EnclaveMetadata, Time};
    use sgx_types::{sgx_enclave_id_t, sgx_status_t};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;
    use store::memory::MemStore;
    use store::KVStore;

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

            let prev_height = Some(input.base_state.prev_height);
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

    fn seed_canonical_base_state(
        app: &AppService<FakeEnclave, MemStore>,
        client_id: &str,
        base_state: &ExplicitStateRef,
    ) {
        let prev_height = base_state.prev_height.expect("test base prev_height");
        let client_state = base_state
            .client_state
            .as_ref()
            .expect("test base client_state");
        let consensus_state = base_state
            .consensus_state
            .as_ref()
            .expect("test base consensus_state");
        let client_state_value =
            bincode::serde::encode_to_vec(client_state, bincode::config::standard())
                .expect("encode client_state");
        let consensus_state_value =
            bincode::serde::encode_to_vec(consensus_state, bincode::config::standard())
                .expect("encode consensus_state");
        app.enclave.use_mut_store(|store| {
            store.set(
                lcp_types::store_key::client_state_bytes(client_id),
                client_state_value,
            );
            store.set(
                lcp_types::store_key::consensus_state_bytes(client_id, &prev_height),
                consensus_state_value,
            );
        });
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
    fn stitch_rejects_first_base_state_that_differs_from_canonical_store() {
        let client_id = "07-tendermint-0";
        let enclave = FakeEnclave::new(Duration::from_millis(1));
        let app = AppService::<FakeEnclave, MemStore>::new("test-home", enclave);
        let service = SpeculativeService::new(1);
        let req = with_explicit_base_state_payload(mk_req(
            "unit-0000",
            client_id,
            Some(Height::new(0, 10)),
            None,
        ));
        let result = SpeculativeUpdateClientResult {
            response: MsgUpdateClientResponse::default(),
            write_set: WriteSet::default(),
            base_state: req.base_state.clone(),
            observed_transition: ObservedStateTransition {
                prev_height: Some(Height::new(0, 10)),
                prev_state_id: None,
                post_height: Height::new(0, 11),
                post_state_id: vec![1; 32],
            },
        };

        let err = service
            .stitch_speculative_update_client_batch(
                &app,
                SpeculativeUpdateClientBatch {
                    client_id: client_id.to_string(),
                    units: vec![req],
                },
                SpeculativeUpdateClientBatchResult {
                    client_id: client_id.to_string(),
                    units: vec![result],
                },
            )
            .expect_err("non-canonical first base state must be rejected");

        assert_eq!(err.kind, SpeculativeBatchFailureKind::BaseStateMismatch);
        assert_eq!(err.unit_id.as_deref(), Some("unit-0000"));
        assert!(
            err.detail
                .contains("canonical speculative base client_state mismatch"),
            "unexpected error detail: {}",
            err.detail
        );
    }

    #[test]
    fn streaming_speculative_batch_executes_before_input_closes() {
        let client_id = "07-tendermint-0";
        let enclave = FakeEnclave::new(Duration::from_millis(100));
        let app = AppService::<FakeEnclave, MemStore>::new("test-home", enclave);
        let service = SpeculativeService::new(1);
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let worker_service = service.clone();
        let worker_app = app.clone();
        let client_id_for_worker = client_id.to_string();
        let handle = thread::spawn(move || {
            worker_service.execute_speculative_update_client_stream(
                &worker_app,
                client_id_for_worker,
                rx,
            )
        });

        let first_req = with_explicit_base_state_payload(SpeculativeUpdateClientRequest {
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
                prev_height: Some(Height::new(0, 10)),
                prev_state_id: None,
                client_state: None,
                consensus_state: None,
            },
        });
        seed_canonical_base_state(&app, client_id, &first_req.base_state);
        tx.send(ResidentSpeculativeUpdateClientRequest::unmetered(first_req))
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
            with_explicit_base_state_payload(SpeculativeUpdateClientRequest {
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
                    prev_height: Some(Height::new(0, 11)),
                    prev_state_id: Some({
                        let mut prev_state_id = vec![0; 32];
                        prev_state_id[31] = 1;
                        prev_state_id
                    }),
                    client_state: None,
                    consensus_state: None,
                },
            }),
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
    fn streaming_speculative_batch_rejects_incomplete_base_state() {
        let client_id = "07-tendermint-0";
        let enclave = FakeEnclave::new(Duration::from_millis(1));
        let app = AppService::<FakeEnclave, MemStore>::new("test-home", enclave);
        let service = SpeculativeService::new(2);
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let worker_service = service.clone();
        let worker_app = app.clone();
        let client_id_for_worker = client_id.to_string();
        let handle = thread::spawn(move || {
            worker_service.execute_speculative_update_client_stream(
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
        drop(tx);

        let err = handle
            .join()
            .expect("streaming worker thread")
            .expect_err("incomplete base state should fail");
        assert_eq!(err.kind, SpeculativeBatchFailureKind::BaseStateMismatch);
        assert_eq!(err.unit_id.as_deref(), Some("unit-0000"));
        assert!(
            err.detail.contains("complete base_state payload"),
            "unexpected error detail: {}",
            err.detail
        );
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
            worker_service.execute_speculative_update_client_stream(
                &worker_app,
                client_id_for_worker,
                rx,
            )
        });

        let mut requests = vec![
            with_explicit_base_state_payload(mk_req(
                "unit-0000",
                client_id,
                Some(Height::new(0, 10)),
                None,
            )),
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
        seed_canonical_base_state(&app, client_id, &requests[0].base_state);
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

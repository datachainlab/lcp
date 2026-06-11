use crate::client_lock::ClientUpdateLocks;
use crate::ecall_pool::EcallPool;
use crate::speculative::SpeculativeService;
use anyhow::Result;
use enclave_api::{EnclaveProtoAPI, SpeculativeEnclaveCommandAPI};
use lcp_proto::lcp::service::{
    elc::v1::{msg_server::MsgServer as ELCMsgServer, query_server::QueryServer as ELCQueryServer},
    enclave::v1::query_server::QueryServer as EnclaveQueryServer,
};
use log::*;
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, sync::Arc};
use store::transaction::{CommitStore, TxAccessor};
use tokio::signal::unix::{signal, SignalKind};
use tonic::transport::Server;

pub struct AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    pub(crate) home: PathBuf,
    pub(crate) enclave: Arc<E>,
    /// Long-lived pool that owns the set of OS threads allowed to ECALL.
    /// All ECALL-issuing call sites in the gRPC layer dispatch through this
    /// pool to keep the cumulative set of distinct host threads that ever
    /// enter the enclave bounded by `--max-enclave-concurrency`, which is
    /// the invariant TCSPolicy=BIND requires.
    pub(crate) ecall_pool: Arc<EcallPool>,
    _marker: PhantomData<S>,
}

pub struct ElcService<E, S>
where
    S: CommitStore + TxAccessor + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + 'static,
{
    pub(crate) app: AppService<E, S>,
    pub(crate) speculative: SpeculativeService,
    client_update_locks: Arc<ClientUpdateLocks>,
}

impl<E, S> Clone for AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            home: self.home.clone(),
            enclave: self.enclave.clone(),
            ecall_pool: self.ecall_pool.clone(),
            _marker: Default::default(),
        }
    }
}

impl<E, S> Clone for ElcService<E, S>
where
    S: CommitStore + TxAccessor + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            speculative: self.speculative.clone(),
            client_update_locks: self.client_update_locks.clone(),
        }
    }
}

impl<E, S> AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    pub fn new<P: Into<PathBuf>>(home: P, enclave: E, ecall_concurrency: usize) -> Self {
        AppService {
            home: home.into(),
            enclave: Arc::new(enclave),
            ecall_pool: Arc::new(EcallPool::new(ecall_concurrency)),
            _marker: Default::default(),
        }
    }
}

impl<E, S> ElcService<E, S>
where
    S: CommitStore + TxAccessor + 'static,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S> + 'static,
{
    pub fn new<P: Into<PathBuf>>(
        home: P,
        enclave: E,
        speculative_concurrency_limit: usize,
        ecall_concurrency: usize,
    ) -> Self {
        let app = AppService::new(home, enclave, ecall_concurrency);
        let speculative = SpeculativeService::new(speculative_concurrency_limit);
        Self {
            app,
            speculative,
            client_update_locks: Arc::new(ClientUpdateLocks::default()),
        }
    }

    pub(crate) fn with_client_update_serialized<T>(
        &self,
        client_id: &str,
        f: impl FnOnce() -> T,
    ) -> T {
        // This lock is intentionally owned by the ELC service, not by the
        // speculative executor: it serializes all canonical UpdateClient writes
        // for a client, including both ordinary gRPC updates and speculative
        // batch stitch commits.
        self.client_update_locks
            .with_client_serialized(client_id, f)
    }
}

pub async fn run_service<E, S>(srv: ElcService<E, S>, addr: SocketAddr) -> Result<()>
where
    S: CommitStore + TxAccessor,
    E: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S>,
{
    let app = srv.app.clone();
    let elc_msg_srv = ELCMsgServer::new(srv.clone());
    let elc_query_srv = ELCQueryServer::new(app.clone());
    let enclave_srv = EnclaveQueryServer::new(app);
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(lcp_proto::FILE_DESCRIPTOR_SET)
        .build()
        .expect("failed to create gRPC reflection servicer");

    let mut sigint = signal(SignalKind::interrupt()).expect("failed to set SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to set SIGTERM handler");
    let shutdown_signal = async {
        let signal_type = tokio::select! {
            _ = sigint.recv() => "SIGINT",
            _ = sigterm.recv() => "SIGTERM",
        };
        info!(
            "shutdown signal ({}) received, stopping server",
            signal_type
        );
    };
    Server::builder()
        .add_service(elc_msg_srv)
        .add_service(elc_query_srv)
        .add_service(enclave_srv)
        .add_service(reflection)
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;
    info!("server stopped");
    Ok(())
}

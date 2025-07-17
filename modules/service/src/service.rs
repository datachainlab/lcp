use anyhow::Result;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::{
    elc::v1::{msg_server::MsgServer as ELCMsgServer, query_server::QueryServer as ELCQueryServer},
    enclave::v1::query_server::QueryServer as EnclaveQueryServer,
};
use log::*;
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, sync::Arc};
use store::transaction::CommitStore;
use tokio::signal::unix::{signal, SignalKind};
use tonic::transport::Server;

pub struct AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    pub(crate) home: PathBuf,
    pub(crate) enclave: Arc<E>,
    _marker: PhantomData<S>,
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
            _marker: Default::default(),
        }
    }
}

impl<E, S> AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    pub fn new<P: Into<PathBuf>>(home: P, enclave: E) -> Self {
        AppService {
            home: home.into(),
            enclave: Arc::new(enclave),
            _marker: Default::default(),
        }
    }
}

pub async fn run_service<E, S>(srv: AppService<E, S>, addr: SocketAddr) -> Result<()>
where
    S: CommitStore,
    E: EnclaveProtoAPI<S>,
{
    let elc_msg_srv = ELCMsgServer::new(srv.clone());
    let elc_query_srv = ELCQueryServer::new(srv.clone());
    let enclave_srv = EnclaveQueryServer::new(srv);
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

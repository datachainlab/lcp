use anyhow::Result;
use enclave_api::Enclave;
use lcp_proto::lcp::service::{
    elc::v1::msg_server::MsgServer as ELCMsgServer,
    enclave::v1::query_server::QueryServer as EnclaveQueryServer,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;
use tonic::transport::Server;

#[derive(Clone)]
pub struct AppService {
    pub(crate) home: PathBuf,
    pub(crate) enclave: Enclave,
}

impl AppService {
    pub fn new<P: Into<PathBuf>>(home: P, enclave: Enclave) -> Self {
        AppService {
            home: home.into(),
            enclave,
        }
    }
}

pub fn run_service(srv: AppService, rt: Arc<Runtime>, addr: SocketAddr) -> Result<()> {
    let elc_srv = ELCMsgServer::new(srv.clone());
    let enclave_srv = EnclaveQueryServer::new(srv);
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(lcp_proto::FILE_DESCRIPTOR_SET)
        .build()
        .expect("failed to create gRPC reflection servicer");
    rt.block_on(async {
        Server::builder()
            .add_service(elc_srv)
            .add_service(enclave_srv)
            .add_service(reflection)
            .serve(addr)
            .await
            .unwrap();
    });
    Ok(())
}

use anyhow::Result;
use enclave_api::Enclave;
use lcp_proto::lcp::service::{
    elc::v1::msg_server::MsgServer as ELCMsgServer,
    enclave::v1::query_server::QueryServer as EnclaveQueryServer,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::runtime::Runtime;
use tonic::transport::Server;

#[derive(Clone)]
pub struct AppService {
    pub(crate) enclave: Enclave,
}

impl AppService {
    pub fn builder(enclave: Enclave) -> Self {
        AppService { enclave }
    }
}

pub fn run_service(srv: AppService, rt: Arc<Runtime>, addr: SocketAddr) -> Result<()> {
    let elc_srv = ELCMsgServer::new(srv.clone());
    let enclave_srv = EnclaveQueryServer::new(srv);
    rt.block_on(async {
        Server::builder()
            .add_service(elc_srv)
            .add_service(enclave_srv)
            .serve(addr)
            .await
            .unwrap();
    });
    Ok(())
}

use anyhow::Result;
use enclave_api::{Enclave, EnclaveAPI};
use ibc_proto::ibc::core::client::v1::msg_server::MsgServer;
use std::{net::SocketAddr, sync::Arc};
use tokio::runtime::Runtime;
use tonic::transport::Server;

pub struct AppService {
    enclave: Enclave,
}

impl AppService {
    pub fn builder(enclave: Enclave) -> Self {
        AppService { enclave }
    }
}

pub fn run_service(srv: AppService, rt: Arc<Runtime>, addr: SocketAddr) -> Result<()> {
    let srv = MsgServer::new(srv);
    rt.block_on(async {
        Server::builder()
            .add_service(srv)
            .serve(addr)
            .await
            .unwrap();
    });
    Ok(())
}

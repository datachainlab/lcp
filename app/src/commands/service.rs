use crate::enclave::EnclaveLoader;
use crate::opts::{EnclaveOpts, Opts};
use anyhow::Result;
use clap::Parser;
use enclave_api::{Enclave, EnclaveInfo, EnclaveProtoAPI};
use host::store::transaction::CommitStore;
use log::*;
use service::{run_service, AppService};
use std::sync::Arc;
use tokio::runtime::Builder;

// `service` subcommand
#[derive(Debug, Parser)]
pub enum ServiceCmd {
    #[clap(about = "Start the App service")]
    Start(Start),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct Start {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// Address of the App service
    #[clap(
        long = "address",
        default_value = "[::1]:50051",
        help = "Address of the App service"
    )]
    pub address: String,
    /// Worker thread number the tokio `Runtime` will use
    /// This value is recommended to be less than or equal to TCS_NUM in Enclave config.
    #[clap(
        long = "threads",
        help = "Worker thread number the tokio `Runtime` will use"
    )]
    pub threads: Option<usize>,
}

impl ServiceCmd {
    pub fn run<S, L>(&self, opts: &Opts, enclave_loader: L) -> Result<()>
    where
        S: CommitStore + 'static,
        Enclave<S>: EnclaveProtoAPI<S>,
        L: EnclaveLoader<S>,
    {
        match self {
            Self::Start(cmd) => {
                let addr = cmd.address.parse()?;
                let enclave =
                    enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?;
                let metadata = enclave.metadata()?;
                let mrenclave = metadata.mrenclave().to_hex_string();
                let mut rb = Builder::new_multi_thread();
                let rb = if let Some(threads) = cmd.threads {
                    rb.worker_threads(threads)
                } else {
                    &mut rb
                };
                let rt = Arc::new(rb.enable_all().build()?);
                let srv = AppService::new(opts.get_home(), enclave);

                info!("start service: addr={addr} mrenclave={mrenclave}");
                run_service(srv, rt, addr)
            }
        }
    }
}

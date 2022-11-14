use crate::opts::Opts;
use anyhow::Result;
use clap::Parser;
use enclave_api::{Enclave, EnclaveProtoAPI};
use log::*;
use service::{run_service, AppService};
use std::path::PathBuf;
use std::sync::Arc;
use store::transaction::CommitStore;
use tokio::runtime::Builder;

// `service` subcommand
#[derive(Debug, Parser)]
pub enum ServiceCmd {
    #[clap(about = "Start the App service")]
    Start(Start),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct Start {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,
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
    pub fn run<'e, S>(
        &self,
        opts: &Opts,
        enclave_loader: impl FnOnce(&Opts, Option<&PathBuf>) -> Result<Enclave<'e, S>>,
    ) -> Result<()>
    where
        'e: 'static,
        S: CommitStore + 'e,
        Enclave<'e, S>: EnclaveProtoAPI<S>,
    {
        match self {
            Self::Start(cmd) => {
                let addr = cmd.address.parse()?;
                let enclave = enclave_loader(opts, cmd.enclave.as_ref())?;

                let mut rb = Builder::new_multi_thread();
                let rb = if let Some(threads) = cmd.threads {
                    rb.worker_threads(threads)
                } else {
                    &mut rb
                };
                let rt = Arc::new(rb.enable_all().build()?);
                let srv = AppService::new(opts.get_home(), enclave);

                info!("start service: addr={addr}");
                run_service(srv, rt, addr)
            }
        }
    }
}

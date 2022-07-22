use crate::enclave::load_enclave;
use crate::opts::Opts;
use anyhow::Result;
use clap::Parser;
use log::*;
use service::{run_service, AppService};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

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
}

impl ServiceCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            Self::Start(cmd) => {
                let addr = cmd.address.parse()?;
                let enclave = load_enclave(opts, cmd.enclave.as_ref())?;
                let rt = Arc::new(Runtime::new()?);
                let srv = AppService::builder(enclave);

                info!("start service");
                run_service(srv, rt, addr)
            }
        }
    }
}

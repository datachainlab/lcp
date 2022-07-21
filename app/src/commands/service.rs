use crate::opts::Opts;
use anyhow::{bail, Result};
use clap::Parser;
use enclave_api::Enclave;
use host::enclave::init_enclave;
use log::*;
use service::{run_service, AppService};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

// `enclave` subcommand
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
                let path = if let Some(path) = cmd.enclave.as_ref() {
                    path.clone()
                } else {
                    opts.default_enclave()
                };
                let enclave = match init_enclave(&path) {
                    Ok(r) => {
                        info!(
                            "Init Enclave Successful: eid={} path={:?}",
                            r.geteid(),
                            path.as_path()
                        );
                        r
                    }
                    Err(x) => {
                        bail!(
                            "Init Enclave Failed: status={} path={:?}",
                            x.as_str(),
                            path.as_path()
                        );
                    }
                };

                let rt = Arc::new(Runtime::new()?);
                let enclave = Enclave::new(enclave, opts.get_home().to_str().unwrap().to_string());
                let srv = AppService::builder(enclave);

                log::info!("start service");
                run_service(srv, rt, addr)
            }
        }
    }
}

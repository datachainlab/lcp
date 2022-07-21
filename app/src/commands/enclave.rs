use crate::opts::Opts;
use anyhow::{bail, Result};
use clap::Parser;
use enclave_api::{Enclave, EnclaveAPI};
use host::enclave::init_enclave;
use log::*;
use std::path::PathBuf;

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Initialize an enclave key")]
    InitKey(InitKey),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct InitKey {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,
}

impl EnclaveCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            EnclaveCmd::InitKey(cmd) => {
                let spid = std::env::var("SPID")?;
                let ias_key = std::env::var("IAS_KEY")?;

                let home = opts.get_home();
                if !home.exists() {
                    info!("create home directory: {:?}", home);
                    std::fs::create_dir_all(&home)?;
                }

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

                if let Err(e) = Enclave::new(enclave, home.to_str().unwrap().to_string())
                    .init_enclave_key(spid.as_bytes(), ias_key.as_bytes())
                {
                    bail!("ECALL Enclave Failed {:?}!", e);
                } else {
                    info!("remote attestation success...");
                }

                Ok(())
            }
        }
    }
}

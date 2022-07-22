use crate::{enclave::load_enclave, opts::Opts};
use anyhow::{bail, Result};
use clap::Parser;
use enclave_api::EnclaveAPI;
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

                let enclave = load_enclave(opts, cmd.enclave.as_ref())?;
                if let Err(e) = enclave.init_enclave_key(spid.as_bytes(), ias_key.as_bytes()) {
                    bail!("ECALL Enclave Failed {:?}!", e);
                } else {
                    info!("remote attestation success...");
                }

                Ok(())
            }
        }
    }
}

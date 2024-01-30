use crate::{
    enclave::EnclaveLoader,
    opts::{EnclaveOpts, Opts},
};
use anyhow::Result;
use clap::Parser;
use enclave_api::{Enclave, EnclaveProtoAPI};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use store::transaction::CommitStore;

// `client` subcommand
#[derive(Debug, Parser)]
pub enum ELCCmd {
    #[clap(display_order = 1, about = "Create Light Client")]
    CreateClient(ELCOpts),
    #[clap(display_order = 2, about = "Update Light Client")]
    UpdateClient(ELCOpts),
}

impl ELCCmd {
    fn opts(&self) -> &ELCOpts {
        match self {
            ELCCmd::CreateClient(opts) => opts,
            ELCCmd::UpdateClient(opts) => opts,
        }
    }
}

#[derive(Clone, Debug, Parser)]
pub struct ELCOpts {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// Path to the proto msg
    #[clap(long = "msg", help = "Path to proto msg")]
    pub msg: PathBuf,
}

impl ELCOpts {
    fn load<T: DeserializeOwned>(&self) -> Result<T> {
        let bz = std::fs::read(&self.msg)?;
        Ok(serde_json::from_slice(&bz)?)
    }
}

impl ELCCmd {
    pub fn run<S, L>(&self, opts: &Opts, enclave_loader: L) -> Result<()>
    where
        S: CommitStore,
        Enclave<S>: EnclaveProtoAPI<S>,
        L: EnclaveLoader<S>,
    {
        let elc_opts = self.opts();
        let enclave = enclave_loader.load(
            opts,
            elc_opts.enclave.path.as_ref(),
            elc_opts.enclave.is_debug(),
        )?;
        match self {
            Self::CreateClient(_) => {
                let _ = enclave.proto_create_client(elc_opts.load()?)?;
            }
            Self::UpdateClient(_) => {
                let _ = enclave.proto_update_client(elc_opts.load()?)?;
            }
        }
        Ok(())
    }
}

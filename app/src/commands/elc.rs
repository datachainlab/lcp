use crate::enclave::load_enclave;
use crate::opts::Opts;
use anyhow::Result;
use clap::Parser;
use serde::de::DeserializeOwned;
use std::path::PathBuf;

// `client` subcommand
#[derive(Debug, Parser)]
pub enum ELCCmd {
    #[clap(about = "Create Light Client")]
    CreateClient(CreateClient),
    #[clap(about = "Update Light Client")]
    UpdateClient(UpdateClient),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct CreateClient {
    #[clap(flatten)]
    pub opts: ELCOpts,
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct UpdateClient {
    #[clap(flatten)]
    pub opts: ELCOpts,
}

// TODO embed it into parent struct
#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ELCOpts {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,
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
    pub fn run(&self, opts: &Opts) -> Result<()> {
        // TODO init the enclave

        match self {
            Self::CreateClient(cmd) => {
                // let msg = cmd.opts.load()?;
                let enclave = load_enclave(opts, cmd.opts.enclave.as_ref())?;
                Ok(())
            }
            Self::UpdateClient(cmd) => Ok(()),
        }
    }
}

use self::{enclave::EnclaveCmd, service::ServiceCmd};
use crate::opts::Opts;
use anyhow::Result;
use clap::Parser;

mod enclave;
mod service;

/// Cli Subcommands
#[derive(Parser, Debug)]
pub enum CliCmd {
    #[clap(subcommand, about = "Enclave subcommands")]
    Enclave(EnclaveCmd),
    #[clap(subcommand, about = "Service subcommands")]
    Service(ServiceCmd),
}

impl CliCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            CliCmd::Enclave(cmd) => cmd.run(opts),
            CliCmd::Service(cmd) => cmd.run(opts),
        }
    }
}

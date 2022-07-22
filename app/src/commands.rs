use self::{elc::ELCCmd, enclave::EnclaveCmd, service::ServiceCmd};
use crate::opts::Opts;
use anyhow::Result;
use clap::Parser;

mod elc;
mod enclave;
mod service;

/// Cli Subcommands
#[derive(Parser, Debug)]
pub enum CliCmd {
    #[clap(subcommand, display_order = 1, about = "Enclave subcommands")]
    Enclave(EnclaveCmd),
    #[clap(subcommand, display_order = 2, about = "ELC subcommands")]
    ELC(ELCCmd),
    #[clap(subcommand, display_order = 3, about = "Service subcommands")]
    Service(ServiceCmd),
}

impl CliCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            CliCmd::Enclave(cmd) => cmd.run(opts),
            CliCmd::Service(cmd) => cmd.run(opts),
            CliCmd::ELC(cmd) => cmd.run(opts),
        }
    }
}

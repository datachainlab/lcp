use crate::{commands::CliCmd, opts::Opts};
use anyhow::Result;
use clap::Parser;

/// Entry point for LCP CLI.
#[derive(Debug, Parser)]
#[cfg_attr(feature = "sgx-sw", clap(
    name = env!("CARGO_PKG_NAME"),
    version = concat!(env!("LCP_VERSION"), "-sw"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
))]
#[cfg_attr(not(feature = "sgx-sw"), clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("LCP_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
))]
pub struct Cli {
    #[clap(flatten)]
    pub opts: Opts,
    #[clap(subcommand)]
    pub command: CliCmd,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        self.command.run(&self.opts)
    }
}

use crate::{commands::CliCmd, opts::Opts};
use anyhow::Result;
use clap::Parser;

/// Entry point for LCP CLI.
#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = concat!(env!("CARGO_PKG_VERSION"), "-", env!("SGX_MODE")),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
)]
pub struct Cli {
    #[clap(flatten)]
    pub opts: Opts,

    /// Subcommand to execute.
    ///
    /// The `command` option will delegate option parsing to the command type,
    /// starting at the first free argument.
    #[clap(subcommand)]
    pub command: CliCmd,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        self.command.run(&self.opts)
    }
}

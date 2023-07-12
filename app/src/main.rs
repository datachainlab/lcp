use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;
mod commands;
mod enclave;
mod opts;

fn main() -> Result<()> {
    Cli::parse().run()
}

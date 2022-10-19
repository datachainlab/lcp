use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;
mod commands;
mod enclave;
mod opts;

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    Cli::parse().run()
}

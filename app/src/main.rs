use anyhow::Result;
use clap::Parser;
use cli::Cli;
use host_environment::Environment;

mod cli;
mod commands;
mod enclave;
mod opts;

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let env = Environment::new();
    host::set_environment(env).unwrap();
    Cli::parse().run()
}

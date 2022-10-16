use anyhow::Result;
use clap::Parser;
use cli::Cli;
use host_environment::Environment;
use ocall_handler::HostOCallHandler;
use once_cell::sync::Lazy;

mod cli;
mod commands;
mod enclave;
mod opts;

fn main() -> Result<()> {
    static CLI_ENV: Lazy<(cli::Cli, Environment)> = Lazy::new(|| {
        let cli = Cli::parse();
        let env = Environment::new();
        (cli, env)
    });

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let (cli, env) = &*CLI_ENV;

    let handler = Box::new(HostOCallHandler::new(env));
    host::ocalls::set_ocall_handler(handler).unwrap();
    cli.run()
}

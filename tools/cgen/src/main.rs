use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;
mod gen;
mod types;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}

use crate::gen::{CGenConfig, CGenSuite, Command};
use anyhow::Result;
use clap::Parser;
use enclave_api::Enclave;
use host_environment::Environment;
use ibc_test_framework::prelude::run_binary_channel_test;
use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, RwLock},
};
use store::{host::HostStore, memory::MemStore};
use tempdir::TempDir;

/// Entry point for LCP CLI.
#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
)]
pub struct Cli {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: PathBuf,
    /// Output path to LCP commitments
    #[clap(long = "out", help = "Output path to LCP commitments")]
    pub out_dir: PathBuf,
    /// Commands to process
    #[clap(long = "commands", help = "Commands to process", multiple = true)]
    pub commands: Vec<String>,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let tmp_dir = TempDir::new("lcp")?;
        let home = tmp_dir.path().to_str().unwrap().to_string();

        let spid = std::env::var("SPID")?.as_bytes().to_vec();
        let ias_key = std::env::var("IAS_KEY")?.as_bytes().to_vec();

        host::set_environment(Environment::new(
            home.into(),
            Arc::new(RwLock::new(HostStore::Memory(MemStore::default()))),
        ))
        .unwrap();

        let env = host::get_environment().unwrap();
        let enclave = Enclave::create(self.enclave, &env.home, env.store.clone())?;

        let mut commands = vec![];
        for c in self.commands {
            commands.push(Command::from_str(&c)?);
        }
        run_binary_channel_test(&CGenSuite::new(
            CGenConfig {
                spid,
                ias_key,
                out_dir: self.out_dir,
            },
            enclave,
            commands,
        ))?;
        Ok(())
    }
}

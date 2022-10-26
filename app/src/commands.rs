use self::{elc::ELCCmd, enclave::EnclaveCmd, service::ServiceCmd};
use crate::{enclave::build_enclave_loader, opts::Opts};
use anyhow::Result;
use clap::Parser;
use host_environment::Environment;
use std::sync::{Arc, RwLock};
use store::{host::HostStore, rocksdb::RocksDBStore};

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
    pub fn run(self, opts: &Opts) -> Result<()> {
        let store = HostStore::RocksDB(RocksDBStore::open(opts.get_store_path()));
        let enclave_loader = build_enclave_loader::<RocksDBStore>();
        let env = Environment::new(opts.get_home(), Arc::new(RwLock::new(store)));
        host::set_environment(env).unwrap();
        match self {
            CliCmd::Enclave(cmd) => cmd.run(opts, enclave_loader),
            CliCmd::Service(cmd) => cmd.run(opts, enclave_loader),
            CliCmd::ELC(cmd) => cmd.run(opts, enclave_loader),
        }
    }
}

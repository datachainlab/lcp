use self::{attestation::AttestationCmd, elc::ELCCmd, enclave::EnclaveCmd, service::ServiceCmd};
use crate::{enclave::build_enclave_loader, opts::Opts};
use anyhow::Result;
use clap::Parser;
use host::environment::store::{host::HostStore, rocksdb::RocksDBStore};
use host::environment::Environment;
use std::sync::{Arc, RwLock};

mod attestation;
mod elc;
mod enclave;
mod service;

/// Cli Subcommands
#[allow(clippy::upper_case_acronyms)]
#[derive(Parser, Debug)]
pub enum CliCmd {
    #[clap(subcommand, display_order = 1, about = "Enclave subcommands")]
    Enclave(EnclaveCmd),
    #[clap(subcommand, display_order = 2, about = "Attestation subcommands")]
    Attestation(AttestationCmd),
    #[clap(subcommand, display_order = 3, about = "ELC subcommands")]
    ELC(ELCCmd),
    #[clap(subcommand, display_order = 4, about = "Service subcommands")]
    Service(ServiceCmd),
}

impl CliCmd {
    pub fn run(self, opts: &Opts) -> Result<()> {
        Self::setup_logger(opts)?;
        match self {
            CliCmd::Enclave(cmd) => {
                Self::setup_read_only_env(opts);
                cmd.run(opts, build_enclave_loader::<RocksDBStore>())
            }
            CliCmd::Attestation(cmd) => {
                Self::setup_read_only_env(opts);
                cmd.run(opts, build_enclave_loader::<RocksDBStore>())
            }
            CliCmd::Service(cmd) => {
                Self::setup_env(opts);
                cmd.run(opts, build_enclave_loader::<RocksDBStore>())
            }
            CliCmd::ELC(cmd) => {
                Self::setup_env(opts);
                cmd.run(opts, build_enclave_loader::<RocksDBStore>())
            }
        }
    }

    fn setup_logger(opts: &Opts) -> Result<()> {
        if let Some(level_filter) = opts.get_log_level_filter()? {
            env_logger::builder().filter_level(level_filter).init();
        } else {
            env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
        }
        Ok(())
    }

    fn setup_env(opts: &Opts) {
        let store = HostStore::RocksDB(RocksDBStore::open(opts.get_state_store_path()));
        let env = Environment::new(opts.get_home(), Arc::new(RwLock::new(store)));
        host::set_environment(env).unwrap();
    }

    fn setup_read_only_env(opts: &Opts) {
        let store = HostStore::RocksDB(RocksDBStore::open_read_only(opts.get_state_store_path()));
        let env = Environment::new(opts.get_home(), Arc::new(RwLock::new(store)));
        host::set_environment(env).unwrap();
    }
}

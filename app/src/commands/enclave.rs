use crate::opts::Opts;
use anyhow::{bail, Result};
use clap::Parser;
use ecall_commands::InitEnclaveInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use log::*;
use serde_json::json;
use settings::SEALED_ENCLAVE_KEY_PATH;
use std::{fs::remove_file, path::PathBuf};
use store::transaction::CommitStore;

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Initialize an Enclave Key")]
    InitKey(InitKey),
    #[clap(about = "Print metadata of the enclave")]
    Metadata(Metadata),
}

impl EnclaveCmd {
    pub fn run<S>(
        &self,
        opts: &Opts,
        enclave_loader: impl FnOnce(&Opts, Option<&PathBuf>) -> Result<Enclave<S>>,
    ) -> Result<()>
    where
        S: CommitStore,
        Enclave<S>: EnclaveProtoAPI<S>,
    {
        let home = opts.get_home();
        match self {
            EnclaveCmd::InitKey(cmd) => {
                if !home.exists() {
                    std::fs::create_dir_all(&home)?;
                    info!("created home directory: {:?}", home);
                }
                run_init_key(enclave_loader(opts, cmd.enclave.as_ref())?, home, cmd)
            }
            EnclaveCmd::Metadata(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_print_metadata(opts, cmd)
            }
        }
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct InitKey {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        default_value = "false",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,
}

fn run_init_key<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    home: PathBuf,
    cmd: &InitKey,
) -> Result<()> {
    let ek_path = home.join(SEALED_ENCLAVE_KEY_PATH);
    if ek_path.exists() {
        if cmd.force {
            remove_file(&ek_path)?;
        } else {
            bail!(
                "Init Key Failed: Enclave Key path {:?} already exists",
                ek_path.as_path(),
            );
        }
    }
    match enclave.init_enclave_key(InitEnclaveInput::default()) {
        Ok(_) => Ok(()),
        Err(e) => bail!("Init Enclave Failed {:?}!", e),
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct Metadata {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
}

fn run_print_metadata(opts: &Opts, cmd: &Metadata) -> Result<()> {
    let metadata = host::sgx_get_metadata(cmd.enclave.clone().unwrap_or(opts.default_enclave()))?;
    println!(
        "{}",
        json! {{
            "mrenclave": format!("0x{}", hex::encode(metadata.enclave_css.body.enclave_hash.m))
        }}
    );
    Ok(())
}

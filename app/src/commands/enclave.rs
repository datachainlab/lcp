use crate::opts::Opts;
use anyhow::{anyhow, Result};
use clap::Parser;
use ecall_commands::GenerateEnclaveKeyInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use log::*;
use serde_json::json;
use std::path::PathBuf;
use store::transaction::CommitStore;

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Generate an Enclave Key")]
    GenerateKey(GenerateKey),
    #[clap(about = "Show list of Enclave Keys")]
    ListKeys(ListKeys),
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
        if !home.exists() {
            std::fs::create_dir_all(&home)?;
            info!("created home directory: {:?}", home);
        }
        match self {
            Self::GenerateKey(cmd) => {
                run_generate_key(enclave_loader(opts, cmd.enclave.as_ref())?, cmd)
            }
            Self::ListKeys(cmd) => run_list_keys(enclave_loader(opts, cmd.enclave.as_ref())?, cmd),
            Self::Metadata(cmd) => run_print_metadata(opts, cmd),
        }
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct GenerateKey {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
}

fn run_generate_key<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    _: &GenerateKey,
) -> Result<()> {
    let res = enclave
        .generate_enclave_key(GenerateEnclaveKeyInput::default())
        .map_err(|e| anyhow!("Init Enclave Failed {:?}!", e))?;

    enclave
        .get_key_manager()
        .save((&res.pub_key).into(), res.sealed_ek)?;
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ListKeys {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
}

fn run_list_keys<E: EnclaveCommandAPI<S>, S: CommitStore>(enclave: E, _: &ListKeys) -> Result<()> {
    let list = enclave.get_key_manager().list()?;
    for addr in list {
        println!("{}", addr);
    }
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct Metadata {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
}

fn run_print_metadata(opts: &Opts, cmd: &Metadata) -> Result<()> {
    let metadata = host::sgx_get_metadata(
        cmd.enclave
            .clone()
            .unwrap_or_else(|| opts.default_enclave()),
    )?;
    println!(
        "{}",
        json! {{
            "mrenclave": format!("0x{}", hex::encode(metadata.enclave_css.body.enclave_hash.m))
        }}
    );
    Ok(())
}

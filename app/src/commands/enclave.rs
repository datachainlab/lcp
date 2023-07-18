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
    #[clap(about = "Generate an Enclave Key", display_order = 1)]
    GenerateKey(GenerateKey),
    #[clap(about = "Show list of Enclave Keys", display_order = 2)]
    ListKeys(ListKeys),
    #[clap(about = "Prune Enclave Keys", display_order = 3)]
    PruneKeys(PruneKeys),
    #[clap(about = "Print metadata of the enclave", display_order = 4)]
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
            Self::PruneKeys(cmd) => {
                run_prune_keys(enclave_loader(opts, cmd.enclave.as_ref())?, cmd)
            }
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
        .map_err(|e| anyhow!("failed to generate an enclave key: {:?}", e))?;
    println!("{}", res.pub_key.as_address());
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ListKeys {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
    #[clap(
        long = "available_only",
        short = 'a',
        help = "Show only available keys"
    )]
    pub available_only: bool,
}

fn run_list_keys<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    input: &ListKeys,
) -> Result<()> {
    let km = enclave.get_key_manager();
    let list = if input.available_only {
        km.available_keys(enclave.metadata()?.enclave_css.body.enclave_hash.m.into())?
    } else {
        km.all_keys()?
    };
    if list.is_empty() {
        return Err(anyhow!("no enclave keys found"));
    }

    let mut list_json = Vec::new();
    for eki in list {
        match eki.avr {
            Some(eavr) => {
                let avr = eavr.get_avr()?;
                list_json.push(json! {{
                    "address": eki.address.to_hex_string(),
                    "attested": true,
                    "isv_enclave_quote_status": avr.isv_enclave_quote_status,
                    "advisory_ids": avr.advisory_ids,
                    "attested_at": avr.timestamp
                }});
            }
            None => {
                list_json.push(json! {{
                    "address": eki.address.to_hex_string(),
                    "attested": false,
                }});
            }
        }
    }
    println!("{}", serde_json::to_string(&list_json).unwrap());
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct PruneKeys {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
    /// expiration in seconds from attested_at
    #[clap(long = "expiration", help = "expiration in seconds from attested_at")]
    pub expiration: u64,
}

fn run_prune_keys<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    input: &PruneKeys,
) -> Result<()> {
    let km = enclave.get_key_manager();
    let count = km.prune(input.expiration)?;
    info!("pruned {} expired enclave keys", count);
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

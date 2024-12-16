use crate::{
    enclave::EnclaveLoader,
    opts::{EnclaveOpts, Opts},
};
use anyhow::{anyhow, Result};
use clap::Parser;
use crypto::Address;
use ecall_commands::GenerateEnclaveKeyInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use host::store::transaction::CommitStore;
use lcp_types::Mrenclave;
use log::*;
use serde_json::json;

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
    pub fn run<S, L>(&self, opts: &Opts, enclave_loader: L) -> Result<()>
    where
        S: CommitStore,
        Enclave<S>: EnclaveProtoAPI<S>,
        L: EnclaveLoader<S>,
    {
        let home = opts.get_home();
        if !home.exists() {
            std::fs::create_dir_all(&home)?;
            info!("created home directory: {:?}", home);
        }
        match self {
            Self::GenerateKey(cmd) => run_generate_key(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            Self::ListKeys(cmd) => run_list_keys(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            Self::PruneKeys(cmd) => run_prune_keys(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            Self::Metadata(cmd) => run_print_metadata(opts, cmd),
        }
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct GenerateKey {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// An operator address to perform `registerEnclaveKey` transaction on-chain
    #[clap(
        long = "operator",
        help = "An operator address to perform `registerEnclaveKey` transaction on-c
    hain"
    )]
    pub operator: Option<String>,
    #[clap(long = "target_qe3", help = "Create a report for QE3 instead of QE")]
    pub target_qe3: bool,
}

impl GenerateKey {
    fn get_operator(&self) -> Result<Option<Address>> {
        if let Some(operator) = &self.operator {
            Ok(Some(Address::from_hex_string(operator)?))
        } else {
            Ok(None)
        }
    }
}

fn run_generate_key<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    input: &GenerateKey,
) -> Result<()> {
    let (target_info, _) = remote_attestation::init_quote(input.target_qe3)?;
    let res = enclave
        .generate_enclave_key(GenerateEnclaveKeyInput {
            target_info,
            operator: input.get_operator()?,
        })
        .map_err(|e| anyhow!("failed to generate an enclave key: {:?}", e))?;
    println!("{}", res.pub_key.as_address());
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ListKeys {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
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
    let mut list_json = Vec::new();
    for eki in list {
        let ias_attested = eki.ias_report.is_some();
        let dcap_attested = eki.dcap_quote.is_some();

        if ias_attested {
            let avr = eki.ias_report.as_ref().unwrap().get_avr()?;
            let report_data = avr.parse_quote()?.report_data();
            list_json.push(json! {{
                "type": "ias",
                "address": eki.address.to_hex_string(),
                "attested": true,
                "report_data": report_data.to_string(),
                "isv_enclave_quote_status": avr.isv_enclave_quote_status,
                "advisory_ids": avr.advisory_ids,
                "attested_at": avr.timestamp
            }});
        } else if dcap_attested {
            let dcap_quote = eki.dcap_quote.as_ref().unwrap();
            list_json.push(json! {{
                "type": "dcap",
                "address": eki.address.to_hex_string(),
                "attested": true,
                "report_data": dcap_quote.report_data()?.to_string(),
                "isv_enclave_quote_status": dcap_quote.tcb_status,
                "advisory_ids": dcap_quote.advisory_ids,
                "attested_at": dcap_quote.attested_at.to_string(),
            }});
        } else {
            list_json.push(json! {{
                "address": eki.address.to_hex_string(),
                "attested": false,
            }});
        }
    }
    println!("{}", serde_json::to_string(&list_json).unwrap());
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct PruneKeys {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
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
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
}

fn run_print_metadata(opts: &Opts, cmd: &Metadata) -> Result<()> {
    let metadata = host::sgx_get_metadata(
        cmd.enclave
            .path
            .clone()
            .unwrap_or_else(|| opts.default_enclave()),
    )?;
    println!(
        "{}",
        json! {{
            "mrenclave": format!("{}", Mrenclave::from(metadata.enclave_css.body.enclave_hash.m)),
        }}
    );
    Ok(())
}

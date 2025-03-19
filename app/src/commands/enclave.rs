use crate::{
    enclave::EnclaveLoader,
    opts::{EnclaveOpts, Opts},
};
use anyhow::{anyhow, Result};
use attestation_report::{QEType, RAQuote};
use clap::Parser;
use crypto::Address;
use ecall_commands::GenerateEnclaveKeyInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use host::store::transaction::CommitStore;
use keymanager::PrunePolicy;
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
    #[clap(
        long = "target_qe",
        default_value = "QE",
        help = "Create a report for the target QE: QE or QE3 or QE3SIM"
    )]
    pub target_qe: QEType,
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
    let (target_info, _) = remote_attestation::get_target_qe_info(input.target_qe)?;
    let res = enclave
        .generate_enclave_key(
            GenerateEnclaveKeyInput {
                target_info,
                operator: input.get_operator()?,
            },
            input.target_qe,
        )
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
        km.available_keys(
            enclave.metadata()?.enclave_css.body.enclave_hash.m.into(),
            input.enclave.is_debug(),
            None,
        )?
    } else {
        km.all_keys()?
    };
    let mut list_json = Vec::new();
    for eki in list {
        match eki.ra_quote.as_ref() {
            Some(ra_quote) => {
                let (report_data, isv_enclave_quote_status, advisory_ids) = match ra_quote {
                    RAQuote::IAS(report) => {
                        let avr = report.get_avr()?;
                        let report_data = avr.parse_quote()?.report_data();
                        (
                            report_data.to_string(),
                            avr.isv_enclave_quote_status,
                            avr.advisory_ids,
                        )
                    }
                    RAQuote::DCAP(quote) => {
                        let report_data = quote.report_data()?.to_string();
                        (
                            report_data,
                            quote.status.clone(),
                            quote.advisory_ids.clone(),
                        )
                    }
                    RAQuote::ZKDCAP(quote) => {
                        let report_data = quote.dcap_quote.report_data()?.to_string();
                        (
                            report_data,
                            quote.dcap_quote.status.clone(),
                            quote.dcap_quote.advisory_ids.clone(),
                        )
                    }
                };
                list_json.push(json! {{
                    "ra_type": ra_quote.ra_type().to_string(),
                    "address": eki.address.to_hex_string(),
                    "qe_type": eki.qe_type.to_string(),
                    "attested": true,
                    "report_data": report_data,
                    "isv_enclave_quote_status": isv_enclave_quote_status,
                    "advisory_ids": advisory_ids,
                    "valid_from": ra_quote.valid_from()?.as_unix_timestamp_secs(),
                    "valid_to": ra_quote.valid_to()?.as_unix_timestamp_secs(),
                }});
            }
            None => {
                list_json.push(json! {{
                    "address": eki.address.to_hex_string(),
                    "qe_type": eki.qe_type.to_string(),
                    "attested": false,
                }});
            }
        }
    }
    println!("{}", serde_json::to_string(&list_json).unwrap());
    Ok(())
}

/// This command prunes expired enclave keys from the key manager.
///
/// The command has two options:
/// 1. `expiration_period` - Prune keys without a `valid_to` timestamp older than this period.
/// 2. `expired_valid_to` - Prune keys with a set `valid_to` timestamp if it is earlier than or equal to the current local time.
///
/// If neither option is specified, the command prunes keys without a `valid_to` timestamp older than 30 days.
#[derive(Clone, Debug, Parser, PartialEq)]
pub struct PruneKeys {
    /// Options for enclave.
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// Expiration period in seconds. Keys without a `valid_to` timestamp older than this period will be pruned.
    /// Default value is 30 days.
    #[clap(long)]
    pub expiration_period: Option<u64>,
    /// Prune keys with a set `valid_to` timestamp if it is earlier than or equal to the current local time.
    #[clap(long)]
    pub expired_valid_to: Option<bool>,
}

fn run_prune_keys<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    input: &PruneKeys,
) -> Result<()> {
    let km = enclave.get_key_manager();
    let count = match (input.expiration_period, input.expired_valid_to) {
        (Some(_), Some(_)) => {
            return Err(anyhow!(
                "Only one of `expiration_period` or `expired_valid_to` can be specified"
            ));
        }
        (None, Some(true)) => km.prune(None, PrunePolicy::ValidTo)?,
        (Some(expiration_period), None) => {
            km.prune(None, PrunePolicy::ExpiredCreatedAt(expiration_period))?
        }
        _ => km.prune(None, PrunePolicy::ExpiredCreatedAt(30 * 24 * 60 * 60))?,
    };
    info!("Pruned {} expired enclave keys", count);
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

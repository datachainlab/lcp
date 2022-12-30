use crate::opts::Opts;
use anyhow::{bail, Result};
use attestation_report::EndorsedAttestationVerificationReport;
use clap::Parser;
use ecall_commands::{IASRemoteAttestationInput, InitEnclaveInput};
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use log::*;
use settings::{AVR_KEY_PATH, SEALED_ENCLAVE_KEY_PATH};
use std::{
    fs::{read, remove_file, OpenOptions},
    io::Write,
    path::PathBuf,
};
use store::transaction::CommitStore;

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Initialize an Enclave Key")]
    InitKey(InitKey),
    #[clap(about = "Print mrenclave of the enclave")]
    Mrenclave(Mrenclave),
    #[clap(about = "Perform Remote Attestation with IAS")]
    IASRemoteAttestation(IASRemoteAttestation),
    #[clap(about = "Show the AVR info")]
    ShowAVR(ShowAVR),
}

impl EnclaveCmd {
    pub fn run<'e, S>(
        &self,
        opts: &Opts,
        enclave_loader: impl FnOnce(&Opts, Option<&PathBuf>) -> Result<Enclave<'e, S>>,
    ) -> Result<()>
    where
        S: CommitStore,
        Enclave<'e, S>: EnclaveProtoAPI<S>,
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
            EnclaveCmd::Mrenclave(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_print_mrenclave(opts, cmd)
            }
            EnclaveCmd::IASRemoteAttestation(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_ias_remote_attestation(enclave_loader(opts, cmd.enclave.as_ref())?, home, cmd)
            }
            EnclaveCmd::ShowAVR(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_show_avr(opts, home, cmd)
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
pub struct Mrenclave {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
}

fn run_print_mrenclave(opts: &Opts, cmd: &Mrenclave) -> Result<()> {
    let metadata = host::sgx_get_metadata(cmd.enclave.clone().unwrap_or(opts.default_enclave()))?;
    println!(
        "0x{}",
        hex::encode(metadata.enclave_css.body.enclave_hash.m)
    );
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct IASRemoteAttestation {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,
}

fn run_ias_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    home: PathBuf,
    cmd: &IASRemoteAttestation,
) -> Result<()> {
    let spid = std::env::var("SPID")?;
    let ias_key = std::env::var("IAS_KEY")?;

    let avr_path = home.join(AVR_KEY_PATH);
    if avr_path.exists() {
        if cmd.force {
            remove_file(&avr_path)?;
        } else {
            bail!(
                "Init Key Failed: AVR path {:?} already exists",
                avr_path.as_path(),
            );
        }
    }

    match enclave.ias_remote_attestation(IASRemoteAttestationInput {
        spid: spid.as_bytes().to_vec(),
        ias_key: ias_key.as_bytes().to_vec(),
    }) {
        Ok(res) => {
            let s = serde_json::to_string(&res.report)?;
            info!("successfully got the AVR");
            // NOTE: Currently, enclave key and AVR file operations are not atomic.
            // Therefore, if the service is running in the background, the service may read incomplete data (and its attempt will be failed).
            // To solve this problem, consider using the traditional `rename` approach or a File DB such as rocksdb.
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&avr_path)?;
            f.write_all(s.as_bytes())?;
            f.flush()?;
            Ok(())
        }
        Err(e) => bail!("IAS Remote Attestation Failed {:?}!", e),
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ShowAVR {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,
    #[clap(long = "validate", help = "Check if the AVR is valid for the enclave")]
    pub validate: bool,
}

fn run_show_avr(opts: &Opts, home: PathBuf, cmd: &ShowAVR) -> Result<()> {
    let avr_path = home.join(AVR_KEY_PATH);
    if !avr_path.exists() {
        bail!("AVR not found: {:?}", avr_path.as_path());
    }
    let report: EndorsedAttestationVerificationReport =
        serde_json::from_slice(read(avr_path)?.as_slice())?;
    let quote = report.get_avr()?.parse_quote()?;
    let address = quote.get_enclave_key_address()?;
    println!("ENCLAVE_KEY=0x{}", address.to_hex_string());
    if cmd.validate {
        let enclave_path = cmd.enclave.clone().unwrap_or(opts.default_enclave());
        if !enclave_path.exists() {
            bail!("Enclave not found: {:?}", enclave_path.as_path());
        }
        let metadata =
            host::sgx_get_metadata(cmd.enclave.clone().unwrap_or(opts.default_enclave()))?;
        let q_mrenclave = quote.get_mrenclave().m;
        if q_mrenclave == metadata.enclave_css.body.enclave_hash.m {
            println!("MRENCLAVE=0x{}", hex::encode(quote.get_mrenclave().m));
        } else {
            bail!(
                "MRENCLAVE mismatch: expected=0x{} got=0x{}",
                hex::encode(q_mrenclave),
                hex::encode(metadata.enclave_css.body.enclave_hash.m)
            );
        }
    } else {
        println!("MRENCLAVE=0x{}", hex::encode(quote.get_mrenclave().m));
    }

    Ok(())
}

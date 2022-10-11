use crate::{enclave::load_enclave, opts::Opts};
use anyhow::{bail, Result};
use attestation_report::EndorsedAttestationVerificationReport;
use clap::Parser;
use ecall_commands::{IASRemoteAttestationInput, InitEnclaveInput};
use enclave_api::EnclavePrimitiveAPI;
use log::*;
use settings::{AVR_KEY_PATH, SEALED_ENCLAVE_KEY_PATH};
use std::{
    fs::{read, remove_file, OpenOptions},
    io::Write,
    path::PathBuf,
};

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Initialize an Enclave Key")]
    InitKey(InitKey),
    #[clap(about = "Perform Remote Attestation with IAS")]
    IASRemoteAttestation(IASRemoteAttestation),
    #[clap(about = "Show the AVR info")]
    ShowAVR,
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct InitKey {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        default_value = "false",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,
}

fn run_init_key(home: &PathBuf, opts: &Opts, cmd: &InitKey) -> Result<()> {
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
    let enclave = load_enclave(opts, cmd.enclave.as_ref())?;
    match enclave.init_enclave_key(InitEnclaveInput::default()) {
        Ok(_) => Ok(()),
        Err(e) => bail!("Init Enclave Failed {:?}!", e),
    }
}

fn run_ias_remote_attestation(
    home: &PathBuf,
    opts: &Opts,
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

    let enclave = load_enclave(opts, cmd.enclave.as_ref())?;
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
pub struct IASRemoteAttestation {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        default_value = "false",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,
}

impl EnclaveCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            EnclaveCmd::InitKey(cmd) => {
                let home = opts.get_home();
                if !home.exists() {
                    std::fs::create_dir_all(&home)?;
                    info!("created home directory: {:?}", home);
                }
                run_init_key(&home, opts, cmd)
            }
            EnclaveCmd::IASRemoteAttestation(cmd) => {
                let home = opts.get_home();
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_ias_remote_attestation(&home, opts, cmd)
            }
            EnclaveCmd::ShowAVR => {
                let home = opts.get_home();
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                let avr_path = home.join(AVR_KEY_PATH);
                if !avr_path.exists() {
                    bail!("AVR doesn't exist at {:?}", avr_path.as_path());
                }
                let report: EndorsedAttestationVerificationReport =
                    serde_json::from_slice(read(avr_path)?.as_slice())?;
                let quote = report.get_avr()?.parse_quote()?;
                let address = quote.get_enclave_key_address()?;
                println!("ENCLAVE_KEY=0x{}", address.to_hex_string());
                println!("MRENCLAVE=0x{}", hex::encode(quote.get_mrenclave().m));
                Ok(())
            }
        }
    }
}

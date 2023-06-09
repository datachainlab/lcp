use crate::opts::Opts;
use anyhow::{bail, Result};
use attestation_report::EndorsedAttestationVerificationReport;
use clap::Parser;
use ecall_commands::IASRemoteAttestationInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use serde_json::json;
use settings::AVR_KEY_PATH;
use std::{
    fs::{read, remove_file, OpenOptions},
    io::Write,
    path::PathBuf,
};
use store::transaction::CommitStore;

/// `attestation` subcommand
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Parser)]
pub enum AttestationCmd {
    #[clap(display_order = 1, about = "Remote Attestation with IAS")]
    IAS(IASRemoteAttestation),
    #[clap(display_order = 2, about = "Show the AVR info")]
    ShowAVR(ShowAVR),
    #[cfg(feature = "sgx-sw")]
    #[clap(display_order = 3, about = "Simulate Remote Attestation")]
    Simulate(SimulateRemoteAttestation),
}

impl AttestationCmd {
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
            AttestationCmd::IAS(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_ias_remote_attestation(enclave_loader(opts, cmd.enclave.as_ref())?, home, cmd)
            }
            #[cfg(feature = "sgx-sw")]
            AttestationCmd::Simulate(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_simulate_remote_attestation(
                    enclave_loader(opts, cmd.enclave.as_ref())?,
                    home,
                    cmd,
                )
            }
            AttestationCmd::ShowAVR(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_show_avr(opts, home, cmd)
            }
        }
    }
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

    let avr_path = check_and_get_avr_path(home, cmd.force)?;
    match enclave.ias_remote_attestation(IASRemoteAttestationInput {
        spid: spid.as_bytes().to_vec(),
        ias_key: ias_key.as_bytes().to_vec(),
    }) {
        Ok(res) => save_avr(&res.report, &avr_path),
        Err(e) => bail!("failed to perform IAS Remote Attestation: {:?}!", e),
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
    let avr = report.get_avr()?;
    let quote = avr.parse_quote()?;
    if cmd.validate {
        let enclave_path = cmd
            .enclave
            .clone()
            .unwrap_or_else(|| opts.default_enclave());
        if !enclave_path.exists() {
            bail!("Enclave not found: {:?}", enclave_path.as_path());
        }
        let metadata = host::sgx_get_metadata(
            cmd.enclave
                .clone()
                .unwrap_or_else(|| opts.default_enclave()),
        )?;
        quote.match_metadata(&metadata)?;
    }
    println!(
        "{}",
        json! {{
            "mrenclave": format!("0x{}", hex::encode(quote.get_mrenclave().m)),
            "enclave_key": format!("0x{}", quote.get_enclave_key_address()?.to_hex_string()),
            "timestamp": avr.timestamp
        }}
    );
    Ok(())
}

fn check_and_get_avr_path(home: PathBuf, force: bool) -> Result<PathBuf> {
    let avr_path = home.join(AVR_KEY_PATH);
    if avr_path.exists() {
        if force {
            remove_file(&avr_path)?;
        } else {
            bail!(
                "Init Key Failed: AVR path {:?} already exists",
                avr_path.as_path(),
            );
        }
    }
    Ok(avr_path)
}

#[cfg(feature = "sgx-sw")]
#[derive(Clone, Debug, Parser, PartialEq)]
pub struct SimulateRemoteAttestation {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to the enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,

    /// Path to a der-encoded file that contains X.509 certificate
    #[clap(
        long = "signing_cert_path",
        help = "Path to a der-encoded file that contains X.509 certificate"
    )]
    pub signing_cert_path: PathBuf,

    /// Path to a PEM-encoded file that contains PKCS#8 private key
    #[clap(
        long = "signing_key",
        help = "Path to a PEM-encoded file that contains PKCS#8 private key"
    )]
    pub signing_key_path: PathBuf,

    /// Validate a signing certificate using openssl command
    #[clap(
        long = "validate_cert",
        default_value = "true",
        help = "Validate a signing certificate using openssl command"
    )]
    pub validate_cert: bool,

    /// Intel security advisory IDs to include in the report
    #[clap(
        long = "advisory_ids",
        value_delimiter = ',',
        help = "Intel security advisory IDs to include in the report"
    )]
    pub advisory_ids: Vec<String>,

    /// Quote status to include in the report
    #[clap(
        long = "isv_enclave_quote_status",
        default_value = "OK",
        help = "Quote status to include in the report"
    )]
    pub isv_enclave_quote_status: String,
}

#[cfg(feature = "sgx-sw")]
fn run_simulate_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    home: PathBuf,
    cmd: &SimulateRemoteAttestation,
) -> Result<()> {
    use rsa::{
        pkcs1v15::SigningKey,
        pkcs8::DecodePrivateKey,
        signature::{SignatureEncoding, Signer},
        traits::PublicKeyParts,
        RsaPrivateKey,
    };
    use sha2::Sha256;
    use std::fs;

    let pk = RsaPrivateKey::read_pkcs8_pem_file(&cmd.signing_key_path)?;
    let pk_modulus = pk.to_public_key().n().to_bytes_be();
    let signing_key = SigningKey::<Sha256>::new(pk);
    let signing_cert = fs::read(&cmd.signing_cert_path)?;

    if cmd.validate_cert {
        use std::process::Command;
        let ret = Command::new("openssl")
            .args([
                "x509",
                "-noout",
                "-modulus",
                "-inform",
                "der",
                "-in",
                cmd.signing_cert_path.to_str().unwrap(),
            ])
            .output()?;
        if !ret.status.success() {
            bail!(
                "failed to exec openssl command: status={:?} error={:?}",
                ret.status,
                ret.stderr
            )
        }
        let output = String::from_utf8(ret.stdout)?;
        if let Some(modulus) = output.trim().strip_prefix("Modulus=") {
            let modulus =
                hex::decode(modulus).map_err(|e| anyhow::anyhow!("hex decode error: {:?}", e))?;
            if pk_modulus != modulus {
                bail!("modulus mismatch: {:X?} != {:X?}", pk_modulus, modulus)
            }
        } else {
            bail!("unexpected output: {}", output)
        }
    }

    let avr_path = check_and_get_avr_path(home, cmd.force)?;
    let avr =
        match enclave.simulate_remote_attestation(ecall_commands::SimulateRemoteAttestationInput {
            advisory_ids: cmd.advisory_ids.clone(),
            isv_enclave_quote_status: cmd.isv_enclave_quote_status.clone(),
        }) {
            Ok(res) => res.avr,
            Err(e) => bail!("failed to simulate Remote Attestation: {:?}!", e),
        };

    let avr_json = avr.to_canonical_json()?;
    let signature = signing_key.sign(avr_json.as_bytes()).to_vec();
    let eavr = EndorsedAttestationVerificationReport {
        avr: avr_json,
        signature,
        signing_cert,
    };
    save_avr(&eavr, &avr_path)
}

fn save_avr(avr: &EndorsedAttestationVerificationReport, path: &PathBuf) -> Result<()> {
    let s = serde_json::to_string(avr)?;
    // NOTE: Currently, enclave key and AVR file operations are not atomic.
    // Therefore, if the service is running in the background, the service may read incomplete data (and its attempt will be failed).
    // To solve this problem, consider using the traditional `rename` approach or a File DB such as rocksdb.
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(s.as_bytes())?;
    f.flush()?;
    Ok(())
}

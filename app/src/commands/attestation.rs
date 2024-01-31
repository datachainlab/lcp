use crate::{
    enclave::EnclaveLoader,
    opts::{EnclaveOpts, Opts},
};
use anyhow::{bail, Result};
use clap::Parser;
use crypto::Address;
use ecall_commands::IASRemoteAttestationInput;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use store::transaction::CommitStore;

/// `attestation` subcommand
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Parser)]
pub enum AttestationCmd {
    #[clap(display_order = 1, about = "Remote Attestation with IAS")]
    IAS(IASRemoteAttestation),
    #[cfg(feature = "sgx-sw")]
    #[clap(display_order = 2, about = "Simulate Remote Attestation")]
    Simulate(SimulateRemoteAttestation),
}

impl AttestationCmd {
    pub fn run<S, L>(&self, opts: &Opts, enclave_loader: L) -> Result<()>
    where
        S: CommitStore,
        Enclave<S>: EnclaveProtoAPI<S>,
        L: EnclaveLoader<S>,
    {
        let home = opts.get_home();
        match self {
            AttestationCmd::IAS(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_ias_remote_attestation(
                    enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                    cmd,
                )
            }
            #[cfg(feature = "sgx-sw")]
            AttestationCmd::Simulate(cmd) => {
                if !home.exists() {
                    bail!("home directory doesn't exist at {:?}", home);
                }
                run_simulate_remote_attestation(
                    enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                    cmd,
                )
            }
        }
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct IASRemoteAttestation {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// An enclave key attested by Remote Attestation
    #[clap(
        long = "enclave_key",
        help = "An enclave key attested by Remote Attestation"
    )]
    pub enclave_key: String,
}

fn run_ias_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    cmd: &IASRemoteAttestation,
) -> Result<()> {
    let spid = std::env::var("SPID")?;
    let ias_key = std::env::var("IAS_KEY")?;
    let target_enclave_key = Address::from_hex_string(&cmd.enclave_key)?;
    match enclave.ias_remote_attestation(IASRemoteAttestationInput {
        target_enclave_key,
        spid: spid.as_bytes().to_vec(),
        ias_key: ias_key.as_bytes().to_vec(),
    }) {
        Ok(_) => Ok(()),
        Err(e) => bail!("failed to perform IAS Remote Attestation: {:?}!", e),
    }
}

#[cfg(feature = "sgx-sw")]
#[derive(Clone, Debug, Parser, PartialEq)]
pub struct SimulateRemoteAttestation {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,

    /// An enclave key attested by Remote Attestation
    #[clap(
        long = "enclave_key",
        help = "An enclave key attested by Remote Attestation"
    )]
    pub enclave_key: String,

    /// Path to a der-encoded file that contains X.509 certificate
    #[clap(
        long = "signing_cert_path",
        help = "Path to a der-encoded file that contains X.509 certificate"
    )]
    pub signing_cert_path: std::path::PathBuf,

    /// Path to a PEM-encoded file that contains PKCS#8 private key
    #[clap(
        long = "signing_key",
        help = "Path to a PEM-encoded file that contains PKCS#8 private key"
    )]
    pub signing_key_path: std::path::PathBuf,

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
    cmd: &SimulateRemoteAttestation,
) -> Result<()> {
    use enclave_api::rsa::{
        pkcs1v15::SigningKey, pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey,
    };
    use enclave_api::sha2::Sha256;
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

    let target_enclave_key = Address::from_hex_string(&cmd.enclave_key)?;
    match enclave.simulate_remote_attestation(
        ecall_commands::SimulateRemoteAttestationInput {
            target_enclave_key,
            advisory_ids: cmd.advisory_ids.clone(),
            isv_enclave_quote_status: cmd.isv_enclave_quote_status.clone(),
        },
        signing_key,
        signing_cert,
    ) {
        Ok(_) => Ok(()),
        Err(e) => bail!("failed to simulate Remote Attestation: {:?}!", e),
    }
}

use crate::{
    enclave::EnclaveLoader,
    opts::{EnclaveOpts, Opts},
};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use crypto::Address;
use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveProtoAPI};
use host::store::transaction::CommitStore;
use remote_attestation::zkvm::prover::{BonsaiProverOptions, Risc0ProverMode};
use remote_attestation::{dcap, ias, zkdcap, IASMode};
use std::{path::PathBuf, str::FromStr};

/// `attestation` subcommand
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Parser)]
pub enum AttestationCmd {
    #[clap(display_order = 1, about = "Remote Attestation with IAS")]
    IAS(IASRemoteAttestation),
    #[clap(display_order = 2, about = "Remote Attestation with DCAP")]
    DCAP(DCAPRemoteAttestation),
    #[clap(display_order = 3, about = "Remote Attestation with zkDCAP")]
    ZKDCAP(ZKDCAPRemoteAttestation),
    #[cfg(feature = "sgx-sw")]
    #[clap(display_order = 4, about = "Simulate Remote Attestation")]
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
        if !home.exists() {
            bail!("home directory doesn't exist at {:?}", home);
        }
        match self {
            AttestationCmd::IAS(cmd) => run_ias_remote_attestation(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            AttestationCmd::DCAP(cmd) => run_dcap_remote_attestation(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            AttestationCmd::ZKDCAP(cmd) => run_zkdcap_remote_attestation(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
            #[cfg(feature = "sgx-sw")]
            AttestationCmd::Simulate(cmd) => run_simulate_remote_attestation(
                enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?,
                cmd,
            ),
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
    /// IAS mode
    #[clap(long = "development", help = "Use IAS development mode")]
    pub is_dev: bool,
}

fn run_ias_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    cmd: &IASRemoteAttestation,
) -> Result<()> {
    let spid = std::env::var("SPID")?;
    let ias_key = std::env::var("IAS_KEY")?;
    let target_enclave_key = Address::from_hex_string(&cmd.enclave_key)?;
    ias::run_ias_ra(
        enclave.get_key_manager(),
        target_enclave_key,
        if cmd.is_dev {
            IASMode::Development
        } else {
            IASMode::Production
        },
        spid,
        ias_key,
    )
    .map_err(|e| anyhow!("failed to perform IAS Remote Attestation: {}", e))?;
    Ok(())
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

    /// An operator address to perform `registerEnclaveKey` transaction on-chain
    #[clap(
        long = "operator",
        help = "An operator address to perform `registerEnclaveKey` transaction on-chain"
    )]
    pub operator: Option<String>,

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
    use remote_attestation::rsa::{
        pkcs1v15::SigningKey, pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey,
    };
    use remote_attestation::sha2::Sha256;
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
    remote_attestation::ias_simulation::run_ias_ra_simulation(
        enclave.get_key_manager(),
        target_enclave_key,
        cmd.advisory_ids.clone(),
        cmd.isv_enclave_quote_status.clone(),
        signing_key,
        signing_cert,
    )
    .map_err(|e| anyhow!("failed to simulate Remote Attestation: {}", e))?;
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct SgxCollateralService {
    #[clap(
        long = "pcss_url",
        help = "PCSS URL (default: https://api.trustedservices.intel.com/)"
    )]
    pub pcss_url: Option<String>,
    #[clap(
        long = "certs_service_url",
        help = "Certs Service URL (default: https://certificates.trustedservices.intel.com/)"
    )]
    pub certs_service_url: Option<String>,
    #[clap(long = "early_update", help = "Enable early update (default: false)")]
    pub is_early_update: bool,
}

impl SgxCollateralService {
    pub fn get_pcss_url(&self) -> String {
        self.pcss_url
            .clone()
            .unwrap_or_else(|| "https://api.trustedservices.intel.com/".to_string())
    }

    pub fn get_certs_service_url(&self) -> String {
        self.certs_service_url
            .clone()
            .unwrap_or_else(|| "https://certificates.trustedservices.intel.com/".to_string())
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct DCAPRemoteAttestation {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// An enclave key attested by Remote Attestation
    #[clap(
        long = "enclave_key",
        help = "An enclave key attested by Remote Attestation"
    )]
    pub enclave_key: String,
    #[clap(flatten)]
    pub collateral_service: SgxCollateralService,
}

fn run_dcap_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    cmd: &DCAPRemoteAttestation,
) -> Result<()> {
    dcap::run_dcap_ra(
        enclave.get_key_manager(),
        Address::from_hex_string(&cmd.enclave_key)?,
        &cmd.collateral_service.get_pcss_url(),
        &cmd.collateral_service.get_certs_service_url(),
        cmd.collateral_service.is_early_update,
    )?;
    Ok(())
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct ZKDCAPRemoteAttestation {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// An enclave key attested by Remote Attestation
    #[clap(
        long = "enclave_key",
        help = "An enclave key attested by Remote Attestation"
    )]
    pub enclave_key: String,
    #[clap(flatten)]
    pub collateral_service: SgxCollateralService,
    #[clap(long = "program_path", help = "Path to the zkVM guest program")]
    pub program_path: Option<PathBuf>,
    #[clap(
        long = "prove_mode",
        default_value = "local",
        help = "Prove mode (dev or local or bonsai)"
    )]
    pub prove_mode: ProveMode,
    #[clap(long = "bonsai_api_url", help = "Bonsai API URL")]
    pub bonsai_api_url: Option<String>,
    #[clap(long = "bonsai_api_key", help = "Bonsai API key")]
    pub bonsai_api_key: Option<String>,
    #[clap(
        long = "disable_pre_execution",
        help = "Disable pre-execution before proving"
    )]
    pub disable_pre_execution: bool,
}

impl ZKDCAPRemoteAttestation {
    pub fn get_zkvm_program(&self) -> Result<Vec<u8>> {
        match &self.program_path {
            Some(path) => std::fs::read(path).map_err(|e| {
                anyhow!(
                    "failed to read zk program: path={} error={}",
                    path.to_string_lossy(),
                    e
                )
            }),
            None => Ok(zkdcap_risc0::DCAP_QUOTE_VERIFIER_ELF.to_vec()),
        }
    }

    pub fn get_bonsai_prover_options(&self) -> Result<BonsaiProverOptions> {
        Ok(BonsaiProverOptions {
            api_url: self.bonsai_api_url.clone(),
            api_key: self.bonsai_api_key.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProveMode {
    Dev,
    Local,
    Bonsai,
}

impl FromStr for ProveMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dev" => Ok(Self::Dev),
            "local" => Ok(Self::Local),
            "bonsai" => Ok(Self::Bonsai),
            _ => Err(anyhow!("invalid prove mode: {}", s)),
        }
    }
}

fn run_zkdcap_remote_attestation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: E,
    cmd: &ZKDCAPRemoteAttestation,
) -> Result<()> {
    let mode = match cmd.prove_mode {
        ProveMode::Dev => Risc0ProverMode::Dev,
        ProveMode::Local => Risc0ProverMode::Local,
        ProveMode::Bonsai => Risc0ProverMode::Bonsai(cmd.get_bonsai_prover_options()?),
    };
    zkdcap::run_zkdcap_ra(
        enclave.get_key_manager(),
        Address::from_hex_string(&cmd.enclave_key)?,
        mode,
        cmd.get_zkvm_program()?.as_ref(),
        cmd.disable_pre_execution,
        &cmd.collateral_service.get_pcss_url(),
        &cmd.collateral_service.get_certs_service_url(),
        cmd.collateral_service.is_early_update,
    )?;
    Ok(())
}

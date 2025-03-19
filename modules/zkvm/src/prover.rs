use crate::Error;
use risc0_zkvm::{
    BonsaiProver, Executor, ExecutorEnv, LocalProver, ProveInfo, Prover, ProverOpts,
    VerifierContext,
};
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Risc0ProverMode {
    /// Mock prover for development
    Dev,
    /// Local prover
    Local,
    /// Bonsai prover
    Bonsai(BonsaiProverOptions),
}

impl Display for Risc0ProverMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Local => write!(f, "local"),
            Self::Bonsai(_) => write!(f, "bonsai"),
        }
    }
}

impl Risc0ProverMode {
    pub fn set_env_vars(&self) -> Result<(), Error> {
        match self {
            Self::Dev => {
                std::env::set_var("RISC0_DEV_MODE", "1");
                Ok(())
            }
            Self::Local => {
                std::env::remove_var("RISC0_DEV_MODE");
                Ok(())
            }
            Self::Bonsai(opts) => {
                std::env::remove_var("RISC0_DEV_MODE");
                std::env::set_var("BONSAI_API_URL", opts.api_url());
                std::env::set_var("BONSAI_API_KEY", opts.api_key()?);
                Ok(())
            }
        }
    }

    pub fn is_dev_mode(&self) -> bool {
        matches!(self, Self::Dev)
    }
}

pub fn prove(mode: &Risc0ProverMode, env: ExecutorEnv, elf: &[u8]) -> Result<ProveInfo, Error> {
    mode.set_env_vars()?;
    match mode {
        Risc0ProverMode::Bonsai(_) => {
            let prover = BonsaiProver::new("bonsai");
            let prover_info = prover
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| Error::bonsai_proving_error(e.to_string()))?;
            Ok(prover_info)
        }
        m => {
            let prover = LocalProver::new(match m {
                Risc0ProverMode::Dev => "dev",
                Risc0ProverMode::Local => "local",
                _ => unreachable!(),
            });
            let prover_info = prover
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| Error::local_proving_error(e.to_string()))?;
            Ok(prover_info)
        }
    }
}

#[derive(Debug, Clone)]
pub struct BonsaiProverOptions {
    // priotize the following order:
    // 1. options
    // 2. env var `BONSAI_API_URL`
    // 3. default to "https://api.bonsai.xyz/"
    pub api_url: Option<String>,
    // priotize the following order:
    // 1. options
    // 2. env var `BONSAI_API_KEY`
    // 3. return error
    pub api_key: Option<String>,
}

impl BonsaiProverOptions {
    pub fn api_url(&self) -> String {
        self.api_url
            .clone()
            .or_else(|| std::env::var("BONSAI_API_URL").ok())
            .unwrap_or_else(|| "https://api.bonsai.xyz/".to_string())
    }

    pub fn api_key(&self) -> Result<String, Error> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("BONSAI_API_KEY").ok())
            .ok_or_else(Error::missing_bonsai_api_key)
    }
}

pub fn get_executor() -> impl Executor {
    LocalProver::new("local")
}

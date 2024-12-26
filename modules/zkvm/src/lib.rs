pub mod errors;
use crate::errors::Error;
pub use risc0_zkvm::{compute_image_id, ExecutorEnv};
use risc0_zkvm::{BonsaiProver, LocalProver, ProveInfo, Prover};

#[derive(Debug, Clone)]
pub enum Risc0ProverMode {
    Local(LocalProverOptions),
    Bonsai(BonsaiProverOptions),
}

impl Risc0ProverMode {
    pub fn set_env_vars(&self) -> Result<(), Error> {
        match self {
            Self::Local(opts) => opts.set_env_vars(),
            Self::Bonsai(opts) => opts.set_env_vars(),
        }
    }
}

pub fn prove(
    prover_type: Risc0ProverMode,
    env: ExecutorEnv,
    elf: &[u8],
) -> Result<ProveInfo, Error> {
    prover_type.set_env_vars()?;
    match prover_type {
        Risc0ProverMode::Local(_) => {
            let prover = LocalProver::new("local");
            let prover_info = prover
                .prove(env, elf)
                .map_err(|e| Error::local_proving_error(e.to_string()))?;
            Ok(prover_info)
        }
        Risc0ProverMode::Bonsai(_) => {
            let prover = BonsaiProver::new("bonsai");
            let prover_info = prover
                .prove(env, elf)
                .map_err(|e| Error::bonsai_proving_error(e.to_string()))?;
            Ok(prover_info)
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalProverOptions {
    // priotize the following order:
    // 1. options
    // 2. env var `RISC0_DEV_MODE`
    // 3. default to false
    pub is_dev_mode: Option<bool>,
}

impl LocalProverOptions {
    pub fn set_env_vars(&self) -> Result<(), Error> {
        std::env::set_var(
            "RISC0_DEV_MODE",
            self.is_dev_mode.unwrap_or(false).to_string(),
        );
        Ok(())
    }

    pub fn is_dev_mode(&self) -> bool {
        self.is_dev_mode.unwrap_or_else(|| {
            std::env::var("RISC0_DEV_MODE")
                .map(|v| v == "1")
                .unwrap_or(false)
        })
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
    pub fn set_env_vars(&self) -> Result<(), Error> {
        std::env::set_var("BONSAI_API_URL", self.api_url());
        std::env::set_var("BONSAI_API_KEY", self.api_key()?);
        Ok(())
    }

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

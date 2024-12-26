mod errors;
pub use crate::errors::Error;
pub use risc0_zkvm::{compute_image_id, ExecutorEnv};
use risc0_zkvm::{sha::Digestible, BonsaiProver, InnerReceipt, LocalProver, ProveInfo, Prover};

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

    pub fn is_dev_mode(&self) -> bool {
        match self {
            Self::Local(opts) => opts.is_dev_mode(),
            Self::Bonsai(_) => false,
        }
    }
}

pub fn prove(mode: &Risc0ProverMode, env: ExecutorEnv, elf: &[u8]) -> Result<ProveInfo, Error> {
    mode.set_env_vars()?;
    match mode {
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

/// Encode the seal of the given receipt for use with EVM smart contract verifiers.
///
/// Appends the verifier selector, determined from the first 4 bytes of the verifier parameters
/// including the Groth16 verification key and the control IDs that commit to the RISC Zero
/// circuits.
pub fn encode_seal(receipt: &risc0_zkvm::Receipt) -> Result<Vec<u8>, Error> {
    let seal = match receipt.inner.clone() {
        InnerReceipt::Fake(receipt) => {
            let seal = receipt.claim.digest().as_bytes().to_vec();
            let selector = &[0u8; 4];
            // Create a new vector with the capacity to hold both selector and seal
            let mut selector_seal = Vec::with_capacity(selector.len() + seal.len());
            selector_seal.extend_from_slice(selector);
            selector_seal.extend_from_slice(&seal);
            selector_seal
        }
        InnerReceipt::Groth16(receipt) => {
            let selector = &receipt.verifier_parameters.as_bytes()[..4];
            // Create a new vector with the capacity to hold both selector and seal
            let mut selector_seal = Vec::with_capacity(selector.len() + receipt.seal.len());
            selector_seal.extend_from_slice(selector);
            selector_seal.extend_from_slice(receipt.seal.as_ref());
            selector_seal
        }
        _ => {
            return Err(Error::unsupported_receipt_type(format!(
                "{:?}",
                receipt.inner
            )))
        }
    };
    Ok(seal)
}

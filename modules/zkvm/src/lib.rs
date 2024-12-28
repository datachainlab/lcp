mod errors;
pub use crate::errors::Error;
use risc0_zkp::verify::VerificationError;
pub use risc0_zkvm;
use risc0_zkvm::{
    sha::{Digest, Digestible},
    BonsaiProver, ExecutorEnv, Groth16Receipt, Groth16ReceiptVerifierParameters, InnerReceipt,
    LocalProver, MaybePruned, ProveInfo, Prover, ProverOpts, ReceiptClaim, VerifierContext,
};

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
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| Error::local_proving_error(e.to_string()))?;
            Ok(prover_info)
        }
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

pub fn create_groth16_receipt(
    seal: Vec<u8>,
    image_id: impl Into<Digest>,
    journal: Vec<u8>,
) -> Groth16Receipt<ReceiptClaim> {
    let claim = MaybePruned::Value(ReceiptClaim::ok(image_id, journal));
    Groth16Receipt::new(
        seal,
        claim,
        Groth16ReceiptVerifierParameters::default().digest(),
    )
}

pub fn verify_groth16_proof(
    seal: Vec<u8>,
    image_id: impl Into<Digest>,
    journal: Vec<u8>,
) -> Result<(), VerificationError> {
    let expected_selector = &seal[..4];
    let data = &seal[4..];
    let receipt = create_groth16_receipt(data.to_vec(), image_id, journal);
    let selector = receipt.verifier_parameters.as_bytes()[..4].to_vec();
    if expected_selector != selector {
        return Err(VerificationError::InvalidProof);
    }
    receipt.verify_integrity()
}

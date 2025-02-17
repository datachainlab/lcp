use crate::{
    dcap::dcap_ra,
    dcap_simulation::{dcap_ra_simulation, DCAPRASimulationOpts},
    dcap_utils::DCAPRemoteAttestationResult,
    errors::Error,
};
use anyhow::anyhow;
use attestation_report::{Risc0ZKVMProof, ZKDCAPQuote, ZKVMProof};
use crypto::Address;
use dcap_quote_verifier::{collaterals::IntelCollateral, verifier::QuoteVerificationOutput};
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use log::*;
use zkvm::{
    encode_seal_selector,
    prover::{get_executor, prove, Risc0ProverMode},
    risc0_zkvm::{compute_image_id, Executor, ExecutorEnv},
    verifier::verify_groth16_proof,
};

#[allow(clippy::too_many_arguments)]
pub fn run_zkdcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    prover_mode: Risc0ProverMode,
    elf: &[u8],
    disable_pre_execution: bool,
    pccs_url: &str,
    certs_server_url: &str,
    is_early_update: bool,
) -> Result<(), Error> {
    let image_id = compute_image_id(elf)
        .map_err(|e| Error::anyhow(anyhow!("cannot compute image id: {}", e)))?;
    info!(
        "run zkDCAP verification with prover_mode={} image_id={} enclave_key={}",
        prover_mode, image_id, target_enclave_key
    );

    let current_time = Time::now();
    let res = dcap_ra(
        key_manager,
        target_enclave_key,
        current_time,
        pccs_url,
        certs_server_url,
        is_early_update,
    )?;

    zkdcap_ra(
        key_manager,
        target_enclave_key,
        prover_mode,
        elf,
        disable_pre_execution,
        current_time,
        res.raw_quote,
        res.collateral,
    )
}

pub fn run_zkdcap_ra_simulation(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    prover_mode: Risc0ProverMode,
    elf: &[u8],
    disable_pre_execution: bool,
    opts: DCAPRASimulationOpts,
) -> Result<(), Error> {
    let image_id = compute_image_id(elf)
        .map_err(|e| Error::anyhow(anyhow!("cannot compute image id: {}", e)))?;
    info!(
        "run zkDCAP simulation with prover_mode={} image_id={} enclave_key={}",
        prover_mode, image_id, target_enclave_key
    );

    let current_time = Time::now();
    let res = dcap_ra_simulation(key_manager, target_enclave_key, current_time, opts)?;

    zkdcap_ra(
        key_manager,
        target_enclave_key,
        prover_mode,
        elf,
        disable_pre_execution,
        current_time,
        res.raw_quote,
        res.collateral,
    )
}

#[allow(clippy::too_many_arguments)]
fn zkdcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    prover_mode: Risc0ProverMode,
    elf: &[u8],
    disable_pre_execution: bool,
    current_time: Time,
    raw_quote: Vec<u8>,
    collateral: IntelCollateral,
) -> Result<(), Error> {
    let image_id = compute_image_id(elf)
        .map_err(|e| Error::anyhow(anyhow!("cannot compute image id: {}", e)))?;

    debug!(
        "proving with input: quote={}, collateral={}, current_time={}",
        hex::encode(&raw_quote),
        hex::encode(collateral.to_bytes()),
        current_time
    );

    if !disable_pre_execution {
        info!("running pre-execution");
        let res = get_executor()
            .execute(
                build_env(
                    &raw_quote,
                    &collateral.to_bytes(),
                    current_time.as_unix_timestamp_secs(),
                )?,
                elf,
            )
            .map_err(|e| Error::anyhow(anyhow!("pre-execution failed: {}", e)))?;
        info!(
            "pre-execution done: exit_code={:?} cycles={} ",
            res.exit_code,
            res.cycles()
        );
    }

    info!("proving with prover mode: {}", prover_mode);
    let now = std::time::Instant::now();
    let prover_info = prove(
        &prover_mode,
        build_env(
            &raw_quote,
            &collateral.to_bytes(),
            current_time.as_unix_timestamp_secs(),
        )?,
        elf,
    )?;
    info!(
        "proving done: elapsed={}secs stats={:?}",
        now.elapsed().as_secs(),
        prover_info.stats
    );

    prover_info
        .receipt
        .verify(image_id)
        .map_err(|e| Error::anyhow(anyhow!("receipt verification failed: {}", e.to_string())))?;
    info!("receipt verified");

    let (selector, seal) = encode_seal_selector(&prover_info.receipt)?;
    if let zkvm::risc0_zkvm::InnerReceipt::Groth16(_) = prover_info.receipt.inner {
        verify_groth16_proof(
            selector,
            seal.clone(),
            image_id,
            prover_info.receipt.journal.bytes.clone(),
        )?;
    } else {
        assert!(
            prover_mode.is_dev_mode(),
            "if not groth16, must be dev mode"
        );
    }

    let output = QuoteVerificationOutput::from_bytes(&prover_info.receipt.journal.bytes)
        .map_err(|e| Error::anyhow(anyhow!("cannot parse receipt: {}", e)))?;

    let quote = DCAPRemoteAttestationResult {
        raw_quote,
        output,
        collateral,
    }
    .get_ra_quote(current_time);
    key_manager
        .save_ra_quote(
            target_enclave_key,
            ZKDCAPQuote::new(
                quote,
                ZKVMProof::Risc0(Risc0ZKVMProof {
                    image_id: image_id.into(),
                    selector,
                    seal,
                    output: prover_info.receipt.journal.bytes,
                }),
            )
            .into(),
        )
        .map_err(|e| {
            Error::key_manager(
                format!("cannot save zkDCAP quote: {}", target_enclave_key),
                e,
            )
        })?;

    Ok(())
}

fn build_env<'a>(
    quote: &[u8],
    collateral: &[u8],
    current_time: u64,
) -> Result<ExecutorEnv<'a>, Error> {
    ExecutorEnv::builder()
        .write(&(quote, collateral, current_time))
        .map_err(|e| Error::anyhow(anyhow!("cannot build env: {}", e)))
        .and_then(|builder| {
            builder
                .build()
                .map_err(|e| Error::anyhow(anyhow!("cannot build env: {}", e)))
        })
}

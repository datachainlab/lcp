use crate::{dcap::dcap_ra, errors::Error};
use anyhow::anyhow;
use attestation_report::{Risc0ZKVMProof, ZKDCAPQuote, ZKVMProof};
use crypto::Address;
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use log::*;
use zkvm::{
    encode_seal,
    prover::{get_executor, prove, Risc0ProverMode},
    risc0_zkvm::{compute_image_id, Executor, ExecutorEnv},
    verifier::verify_groth16_proof,
};

pub fn run_zkdcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    prover_mode: Risc0ProverMode,
    elf: &[u8],
    disable_pre_execution: bool,
    pccs_url: &str,
    certs_server_url: &str,
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
    )?;

    debug!(
        "proving with input: quote={}, collateral={}, current_time={}",
        hex::encode(&res.raw_quote),
        hex::encode(res.collateral.to_bytes()),
        current_time
    );

    if !disable_pre_execution {
        info!("running pre-execution");
        let res = get_executor()
            .execute(
                build_env(
                    &res.raw_quote,
                    &res.collateral.to_bytes(),
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

    info!("proving with prover mode: {:?}", prover_mode);
    let prover_info = prove(
        &prover_mode,
        build_env(
            &res.raw_quote,
            &res.collateral.to_bytes(),
            current_time.as_unix_timestamp_secs(),
        )?,
        elf,
    )?;
    info!("proving done: stats: {:?}", prover_info.stats);

    prover_info
        .receipt
        .verify(image_id)
        .map_err(|e| Error::anyhow(anyhow!("receipt verification failed: {}", e.to_string())))?;
    info!("receipt verified");

    let seal = encode_seal(&prover_info.receipt)?;
    if let zkvm::risc0_zkvm::InnerReceipt::Groth16(_) = prover_info.receipt.inner {
        verify_groth16_proof(
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

    let quote = res.get_quote();
    key_manager
        .save_ra_quote(
            target_enclave_key,
            ZKDCAPQuote::new(
                quote,
                ZKVMProof::Risc0(Risc0ZKVMProof {
                    image_id: image_id.into(),
                    seal,
                    commit: prover_info.receipt.journal.bytes,
                }),
                prover_mode.is_dev_mode(),
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

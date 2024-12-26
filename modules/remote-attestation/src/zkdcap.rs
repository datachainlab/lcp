use crate::{dcap::dcap_ra, errors::Error};
use attestation_report::{Risc0ZKVMProof, ZKDCAPQuote, ZKVMProof};
use crypto::Address;
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use log::info;
use zkvm::{encode_seal, prove, ExecutorEnv, Risc0ProverMode};

pub fn run_zkdcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    prover_mode: Risc0ProverMode,
    elf: &[u8],
) -> Result<(), Error> {
    let current_time = Time::now();
    let res = dcap_ra(key_manager, target_enclave_key, current_time)?;

    let env = ExecutorEnv::builder()
        .write(&(
            res.raw_quote.clone(),
            res.collateral.to_bytes(),
            current_time.as_unix_timestamp_secs(),
        ))
        .unwrap()
        .build()
        .unwrap();

    info!("proving with prover mode: {:?}", prover_mode);
    let prover_info = prove(&prover_mode, env, elf).unwrap();
    info!("prover info: {:?}", prover_info.stats);
    let seal = encode_seal(&prover_info.receipt).unwrap();
    let quote = res.get_quote();

    key_manager
        .save_ra_quote(
            target_enclave_key,
            ZKDCAPQuote::new(
                quote,
                ZKVMProof::Risc0(Risc0ZKVMProof {
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

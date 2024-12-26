use crate::{dcap::dcap_ra, errors::Error};
use crypto::Address;
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use zkvm::{prove, ExecutorEnv, Risc0ProverMode};

pub fn run_zkdcap_ra(
    prover_mode: Risc0ProverMode,
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
) -> Result<(), Error> {
    let current_time = Time::now();
    let res = dcap_ra(key_manager, target_enclave_key, current_time)?;

    let env = ExecutorEnv::builder()
        .write(&(
            res.raw_quote,
            res.collateral.to_bytes(),
            current_time.as_unix_timestamp_secs(),
        ))
        .unwrap()
        .build()
        .unwrap();

    // prove(prover_type, env, &res.elf)

    Ok(())
}

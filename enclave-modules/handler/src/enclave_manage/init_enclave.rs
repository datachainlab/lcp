use crate::enclave_manage::errors::EnclaveManageError as Error;
use anyhow::anyhow;
use enclave_types::commands::{InitEnclaveInput, InitEnclaveResult};
use log::*;
use sgx_types::{sgx_quote_sign_type_t, sgx_spid_t};
use std::format;
use std::string::String;

use attestation_report::verify_report;
use enclave_crypto::KeyManager;
use enclave_remote_attestation::attestation::create_attestation_report;
use enclave_remote_attestation::report::verify_quote_status;
use enclave_utils::storage::write_to_untrusted;
use settings::ENDORSED_ATTESTATION_PATH;

pub fn init_enclave(input: InitEnclaveInput) -> Result<InitEnclaveResult, Error> {
    let spid = decode_spid(&input.spid)?;
    let mut key_manager = KeyManager::new();
    let kp = match key_manager.get_enclave_key() {
        Some(kp) => kp,
        None => key_manager
            .create_enclave_key()
            .map_err(Error::CryptoError)?,
    };
    trace!(
        "ecall_get_attestation_report key pk: {:?}",
        &kp.get_pubkey()
    );
    let report = create_attestation_report(
        kp.get_pubkey().as_report_data(),
        sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE,
        spid,
        &input.ias_key,
    )
    .map_err(Error::SGXError)?;

    verify_report(&report).map_err(Error::SGXError)?;
    verify_quote_status(&report.report).map_err(Error::SGXError)?;
    write_to_untrusted(&report.report, &ENDORSED_ATTESTATION_PATH).map_err(Error::SGXError)?;

    Ok(Default::default())
}

fn decode_spid(hex: &[u8]) -> Result<sgx_spid_t, Error> {
    let mut spid = sgx_spid_t::default();
    let hex = String::from_utf8_lossy(hex);
    let hex = &hex.trim();

    if hex.len() < 16 * 2 {
        Err(Error::OtherError(anyhow!(
            "Input spid file len ({}) is incorrect!",
            hex.len()
        )))
    } else {
        let decoded_vec = hex::decode(hex).unwrap();
        spid.id.copy_from_slice(&decoded_vec[..16]);
        Ok(spid)
    }
}

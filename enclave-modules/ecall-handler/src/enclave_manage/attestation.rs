use crate::enclave_manage::errors::Error;
use crate::prelude::*;
use attestation_report::verify_report;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{CommandParams, IASRemoteAttestationInput, IASRemoteAttestationResult};
use enclave_remote_attestation::{
    attestation::create_attestation_report, report::validate_quote_status,
};
use lcp_types::Time;
use sgx_types::{sgx_quote_sign_type_t, sgx_spid_t};

pub(crate) fn ias_remote_attestation(
    input: IASRemoteAttestationInput,
    params: CommandParams,
) -> Result<IASRemoteAttestationResult, Error> {
    input.validate()?;
    let pub_key = EnclaveKey::unseal(params.sealed_ek)?.get_pubkey();
    let report = {
        let spid = decode_spid(&input.spid);
        let report = create_attestation_report(
            pub_key.as_report_data(),
            sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE,
            spid,
            &input.ias_key,
        )?;
        verify_report(&report, Time::now())?;
        report
    };
    validate_quote_status(&report.get_avr()?)?;
    Ok(IASRemoteAttestationResult { report })
}

#[cfg(feature = "sgx-sw")]
pub(crate) fn simulate_remote_attestation(
    input: ecall_commands::SimulateRemoteAttestationInput,
    params: CommandParams,
) -> Result<ecall_commands::SimulateRemoteAttestationResult, Error> {
    input.validate()?;
    let pub_key = EnclaveKey::unseal(params.sealed_ek)?.get_pubkey();
    let avr = enclave_remote_attestation::simulate::create_attestation_report(
        pub_key.as_report_data(),
        sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE,
        input.advisory_ids,
        input.isv_enclave_quote_status,
    )?;
    validate_quote_status(&avr)?;
    Ok(ecall_commands::SimulateRemoteAttestationResult { avr })
}

// CONTRACT: `hex` length must be 32
fn decode_spid(hex: &[u8]) -> sgx_spid_t {
    assert!(hex.len() == 32);
    let mut spid = sgx_spid_t::default();
    let hex = String::from_utf8_lossy(hex);
    let hex = &hex.trim();

    let decoded_vec = hex::decode(hex).unwrap();
    spid.id.copy_from_slice(&decoded_vec[..16]);
    spid
}

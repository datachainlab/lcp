use crate::enclave_manage::errors::Error;
use crate::prelude::*;
use attestation_report::verify_report;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{CommandContext, IASRemoteAttestationInput, IASRemoteAttestationResult};
use enclave_remote_attestation::{
    attestation::create_attestation_report, report::validate_quote_status,
};
use sgx_types::{sgx_quote_sign_type_t, sgx_spid_t};

pub(crate) fn ias_remote_attestation(
    cctx: CommandContext,
    input: IASRemoteAttestationInput,
) -> Result<IASRemoteAttestationResult, Error> {
    input.validate()?;
    let pub_key =
        EnclaveKey::unseal(&cctx.sealed_ek.ok_or(Error::enclave_key_not_found())?)?.get_pubkey();
    let report = {
        let spid = decode_spid(&input.spid);
        let report = create_attestation_report(
            pub_key.as_report_data(),
            sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE,
            spid,
            &input.ias_key,
        )?;
        verify_report(cctx.current_timestamp, &report)?;
        report
    };
    validate_quote_status(cctx.current_timestamp, &report.get_avr()?)?;
    Ok(IASRemoteAttestationResult { report })
}

#[cfg(feature = "sgx-sw")]
pub(crate) fn simulate_remote_attestation(
    cctx: CommandContext,
    input: ecall_commands::SimulateRemoteAttestationInput,
) -> Result<ecall_commands::SimulateRemoteAttestationResult, Error> {
    input.validate()?;
    let pub_key =
        EnclaveKey::unseal(&cctx.sealed_ek.ok_or(Error::enclave_key_not_found())?)?.get_pubkey();
    let avr = enclave_remote_attestation::simulate::create_attestation_report(
        pub_key.as_report_data(),
        sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE,
        input.advisory_ids,
        input.isv_enclave_quote_status,
    )?;
    validate_quote_status(cctx.current_timestamp, &avr)?;
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

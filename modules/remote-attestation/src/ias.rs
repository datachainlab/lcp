use crate::errors::Error;
use crate::ias_utils::{
    decode_spid, get_quote, get_report_from_intel, get_sigrl_from_intel, init_quote,
    validate_qe_report, IASMode, SGX_QUOTE_SIGN_TYPE,
};
use attestation_report::IASSignedReport;
use crypto::Address;
use keymanager::EnclaveKeyManager;
use log::*;

pub fn run_ias_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    mode: IASMode,
    spid: String,
    ias_key: String,
) -> Result<IASSignedReport, Error> {
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;

    let spid = decode_spid(&spid)?;
    let (target_info, epid_group_id) = init_quote(false)?;
    // Now sigrl is the revocation list, a vec<u8>
    let sigrl = get_sigrl_from_intel(mode, epid_group_id, &ias_key)?;
    let (quote, qe_report) = get_quote(sigrl, ek_info.report, SGX_QUOTE_SIGN_TYPE, spid)?;
    validate_qe_report(&target_info, &qe_report)?;

    let signed_report = get_report_from_intel(mode, quote, &ias_key)?;
    info!("IAS AVR: {:?}", signed_report.avr);
    info!(
        "report_data: {}",
        signed_report.get_avr()?.parse_quote()?.report_data()
    );
    key_manager
        .save_ra_quote(target_enclave_key, signed_report.clone().into())
        .map_err(|e| {
            Error::key_manager(format!("cannot save IAS AVR: {}", target_enclave_key), e)
        })?;
    Ok(signed_report)
}

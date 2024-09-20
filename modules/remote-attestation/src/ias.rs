use crate::errors::Error;
use crate::ias_utils::{
    decode_spid, get_quote, get_report_from_intel, get_sigrl_from_intel, init_quote,
    validate_qe_report, IASMode, SGX_QUOTE_SIGN_TYPE,
};
use attestation_report::IASSignedReport;
use crypto::Address;
use enclave_api::EnclaveCommandAPI;
use log::*;
use store::transaction::CommitStore;

pub fn run_ias_ra<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: &E,
    target_enclave_key: Address,
    mode: IASMode,
    spid: String,
    ias_key: String,
) -> Result<IASSignedReport, Error> {
    let ek_info = enclave
        .get_key_manager()
        .load(target_enclave_key)
        .map_err(|e| {
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
    enclave
        .get_key_manager()
        .save_verifiable_quote(target_enclave_key, signed_report.clone().into())
        .map_err(|e| {
            Error::key_manager(format!("cannot save IAS AVR: {}", target_enclave_key), e)
        })?;
    Ok(signed_report)
}

use crate::errors::Error;
use crate::ias_utils::{
    decode_spid, get_quote, get_report_from_intel, get_sigrl_from_intel, init_quote,
    validate_qe_report, IASMode, SGX_QUOTE_SIGN_TYPE,
};
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use ecall_commands::{CreateReportInput, CreateReportResponse};
use enclave_api::EnclaveCommandAPI;
use store::transaction::CommitStore;

pub fn run_ias_ra<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: &E,
    target_enclave_key: Address,
    operator: Option<Address>,
    mode: IASMode,
    spid: String,
    ias_key: String,
) -> Result<EndorsedAttestationVerificationReport, Error> {
    let spid = decode_spid(&spid);
    let (target_info, epid_group_id) = init_quote()?;
    let CreateReportResponse { report } = enclave
        .create_report(CreateReportInput {
            target_info,
            target_enclave_key,
            operator,
        })
        .unwrap();

    // Now sigrl is the revocation list, a vec<u8>
    let sigrl = get_sigrl_from_intel(mode, epid_group_id, &ias_key)?;
    let (quote, qe_report) = get_quote(sigrl, report, SGX_QUOTE_SIGN_TYPE, spid)?;
    validate_qe_report(&target_info, &qe_report)?;

    get_report_from_intel(mode, quote, &ias_key)
}

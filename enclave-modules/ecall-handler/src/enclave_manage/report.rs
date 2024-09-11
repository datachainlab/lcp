use crate::enclave_manage::Error;
use crate::prelude::*;
use attestation_report::ReportData;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{CommandContext, CreateReportInput, CreateReportResponse};
use sgx_tse::rsgx_create_report;

pub fn create_report(
    cctx: CommandContext,
    input: CreateReportInput,
) -> Result<CreateReportResponse, Error> {
    let pub_key =
        EnclaveKey::unseal(&cctx.sealed_ek.ok_or(Error::enclave_key_not_found())?)?.get_pubkey();
    let report_data = ReportData::new(pub_key.as_address(), input.operator);

    let report = match rsgx_create_report(&input.target_info, &report_data.into()) {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::sgx_error(e, "Report creation => failed".to_string()));
        }
    };
    Ok(CreateReportResponse { report })
}

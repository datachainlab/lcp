use crate::enclave_manage::Error;
use crate::prelude::*;
use attestation_report::ReportData;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{
    EnclaveRuntimeInfo, EnclaveThreadPolicy, GenerateEnclaveKeyInput, GenerateEnclaveKeyResponse,
};
use sgx_trts::enclave::{
    rsgx_get_tcs_max_num, rsgx_get_tcs_num, rsgx_get_thread_policy, rsgx_is_supported_EDMM,
    SgxThreadPolicy,
};
use sgx_tse::rsgx_create_report;

pub(crate) fn generate_enclave_key(
    input: GenerateEnclaveKeyInput,
) -> Result<GenerateEnclaveKeyResponse, Error> {
    let ek = EnclaveKey::new()?;
    let ek_pub = ek.get_pubkey();
    let sealed_ek = ek.seal()?;
    let report_data = ReportData::new(ek_pub.as_address(), input.operator);
    let report = match rsgx_create_report(&input.target_info, &report_data.into()) {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::sgx_error(e, "Report creation => failed".to_string()));
        }
    };
    Ok(GenerateEnclaveKeyResponse {
        pub_key: ek_pub,
        sealed_ek,
        report,
    })
}

pub(crate) fn runtime_info() -> EnclaveRuntimeInfo {
    let (static_tcs_num, eremove_tcs_num, dyn_tcs_num) = rsgx_get_tcs_num();
    EnclaveRuntimeInfo {
        thread_policy: match rsgx_get_thread_policy() {
            SgxThreadPolicy::Bound => EnclaveThreadPolicy::Bound,
            SgxThreadPolicy::Unbound => EnclaveThreadPolicy::Unbound,
        },
        static_tcs_num,
        eremove_tcs_num,
        dyn_tcs_num,
        tcs_max_num: rsgx_get_tcs_max_num(),
        edmm_supported: rsgx_is_supported_EDMM(),
    }
}

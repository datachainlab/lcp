use attestation_report::{
    AttestationVerificationReport, EndorsedAttestationVerificationReport, Quote,
};
use validation_context::ValidationContext;

use crate::client_state::ClientState;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;

// verify_report_and_get_key_expiration
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
pub fn verify_report_and_get_key_expiration(
    vctx: &ValidationContext,
    client_state: &ClientState,
    eavr: &EndorsedAttestationVerificationReport,
) -> (bool, u128) {
    let quote = eavr.get_avr().unwrap().parse_quote().unwrap();

    // TODO verify `avr.signature` with Intel SGX Attestation Report Signing CA

    // check if attestation report's timestamp is not expired
    if vctx.current_timestamp - (quote.timestamp as u128) >= client_state.key_expiration {
        return (false, 0);
    }

    // check if `mr_enclave` that is included in the quote matches the expected value
    if &quote.raw.report_body.mr_enclave.m != client_state.mr_enclave.as_slice() {
        return (false, 0);
    }

    (true, quote.timestamp as u128 + client_state.key_expiration)
}

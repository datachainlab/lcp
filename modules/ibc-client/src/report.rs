use crate::errors::IBCClientError as Error;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use validation_context::ValidationContext;

use crate::client_state::ClientState;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;

// verify_report
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
pub fn verify_report(
    vctx: &ValidationContext,
    client_state: &ClientState,
    eavr: &EndorsedAttestationVerificationReport,
) -> Result<(Address, u128), Error> {
    let quote = eavr.get_avr()?.parse_quote()?;

    // TODO verify `avr.signature` with Intel SGX Attestation Report Signing CA

    // check if attestation report's timestamp is not expired
    let diff = vctx.current_timestamp - (quote.timestamp as u128);
    if diff >= client_state.key_expiration {
        return Err(Error::ExpiredAVRError(diff, client_state.key_expiration));
    }

    // check if `mr_enclave` that is included in the quote matches the expected value
    if &quote.raw.report_body.mr_enclave.m != client_state.mr_enclave.as_slice() {
        return Err(Error::MrenclaveMismatchError(
            quote.raw.report_body.mr_enclave.m.to_vec(),
            client_state.mr_enclave.clone(),
        ));
    }

    Ok((
        quote.get_enclave_key_address()?,
        quote.timestamp as u128 + client_state.key_expiration,
    ))
}

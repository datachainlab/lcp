use crate::client_state::ClientState;
use crate::errors::IBCClientError as Error;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::Address;
use lcp_types::Time;
use validation_context::ValidationContext;

// verify_report
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
pub fn verify_report(
    vctx: &ValidationContext,
    client_state: &ClientState,
    eavr: &EndorsedAttestationVerificationReport,
) -> Result<(Address, Time), Error> {
    // verify AVR with Intel SGX Attestation Report Signing CA
    attestation_report::verify_report(eavr, vctx.current_timestamp)?;

    let quote = eavr.get_avr()?.parse_quote()?;

    // check if attestation report's timestamp is not expired
    let key_expiration = (quote.attestation_time + client_state.key_expiration)?;
    if vctx.current_timestamp > key_expiration {
        return Err(Error::ExpiredAVRError {
            current_timestamp: vctx.current_timestamp,
            attestation_time: quote.attestation_time,
            client_state_key_expiration: client_state.key_expiration,
        });
    }

    // check if `mr_enclave` that is included in the quote matches the expected value
    if &quote.raw.report_body.mr_enclave.m != client_state.mr_enclave.as_slice() {
        return Err(Error::MrenclaveMismatchError(
            quote.raw.report_body.mr_enclave.m.to_vec(),
            client_state.mr_enclave.clone(),
        ));
    }

    Ok((quote.get_enclave_key_address()?, quote.attestation_time))
}

use attestation_report::parse_quote_from_report;
use validation_context::ValidationContext;

use crate::client_state::ClientState;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crypto::Address;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

// AttestationVerificationReport represents Intel's Attestation Verification Report
// https://api.trustedservices.intel.com/documents/sgx-attestation-api-spec.pdf
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AttestationVerificationReport {
    pub body: Vec<u8>,
    pub signature: Vec<u8>,
}

// verify_report_and_get_key_expiration
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
pub fn verify_report_and_get_key_expiration(
    vctx: &ValidationContext,
    client_state: &ClientState,
    avr: &AttestationVerificationReport,
) -> (bool, u128) {
    let quote = parse_quote_from_report(&avr.body).unwrap();

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

// TODO modify Result's right as Error type
// read_enclave_key_from_report parses a report_data from the specified report body and get an enclave key from it
pub fn read_enclave_key_from_report(report_body: &[u8]) -> Result<Address, ()> {
    let quote = parse_quote_from_report(report_body).unwrap();
    let data = quote.raw.report_body.report_data.d;
    Ok(Address::from(&data[..20]))
}

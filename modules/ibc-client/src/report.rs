use attestation_report::parse_quote_from_report;

use crate::crypto::Address;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

// AttestationVerificationReport represents Intel's Attestation Verification Report
// https://api.trustedservices.intel.com/documents/sgx-attestation-api-spec.pdf
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AttestationVerificationReport {
    pub body: Vec<u8>,
    pub signature: Vec<u8>,
}

// verify_report verifies the Attestation Verification Report
pub fn verify_report(expected_mr_enclave: &[u8], avr: &AttestationVerificationReport) -> bool {
    // TODO verify `avr.signature` with Intel SGX Attestation Report Signing CA

    // check if `mr_enclave` that is included in the quote matches the expected value
    let quote = parse_quote_from_report(&avr.body).unwrap();
    if &quote.report_body.mr_enclave.m != expected_mr_enclave {
        return false;
    }
    true
}

// TODO modify Result's right as Error type
// read_enclave_key_from_report parses a report_data from the specified report body and get an enclave key from it
pub fn read_enclave_key_from_report(report_body: &[u8]) -> Result<Address, ()> {
    let quote = parse_quote_from_report(report_body).unwrap();
    let data = quote.report_body.report_data.d;
    Ok(Address::from(&data[..20]))
}

use super::ocalls;
use crate::errors::RemoteAttestationError as Error;
use attestation_report::{AttestationVerificationReport, Quote};
use lcp_types::Time;
use log::*;
use settings::RT_ALLOWED_STATUS;
use sgx_types::{sgx_platform_info_t, sgx_status_t, sgx_update_info_bit_t};
use std::format;
use std::string::ToString;
use std::time::Duration;
use std::vec::Vec;

pub fn validate_quote_status(avr: &AttestationVerificationReport) -> Result<Quote, Error> {
    // 1. Verify quote body
    let quote = avr.parse_quote()?;

    // 2. Check quote's timestamp is within 24H
    let now = Time::now();
    info!("Time: now={:?} quote_timestamp={:?}", now, quote.timestamp);

    if now >= (quote.timestamp + Duration::from_secs(60 * 60 * 24)).unwrap() {
        return Err(Error::TooOldReportTimestampError(format!(
            "The timestamp of the report is too old: now={:?} quote_timestamp={:?}",
            now, quote.timestamp
        )));
    }

    // 3. Verify quote status (mandatory field)
    match quote.status.as_ref() {
        "OK" => (),
        "GROUP_OUT_OF_DATE"
        | "GROUP_REVOKED"
        | "SW_HARDENING_NEEDED"
        | "CONFIGURATION_NEEDED"
        | "CONFIGURATION_AND_SW_HARDENING_NEEDED" => {
            // Verify platformInfoBlob for further info if status not OK
            // https://api.trustedservices.intel.com/documents/sgx-attestation-api-spec.pdf
            // This field is optional, it will only be present if one the following conditions is met:
            // - isvEnclaveQuoteStatus is equal to GROUP_REVOKED, GROUP_OUT_OF_DATE, CONFIGURATION_NEEDED or CONFIGURATION_AND_SW_HARDENING_NEEDED.
            // - pseManifestStatus is equal to one of the following values: OUT_OF_DATE, REVOKED or RL_VERSION_MISMATCH.
            if let Some(pib) = avr.platform_info_blob.as_ref() {
                let mut buf = Vec::new();

                // the TLV Header (4 bytes/8 hexes) should be skipped
                let n = (pib.len() - 8) / 2;
                for i in 0..n {
                    buf.push(u8::from_str_radix(&pib[(i * 2 + 8)..(i * 2 + 10)], 16).unwrap());
                }

                let mut update_info = sgx_update_info_bit_t::default();
                let mut rt: sgx_status_t = sgx_status_t::SGX_ERROR_UNEXPECTED;
                let res = unsafe {
                    ocalls::ocall_get_update_info(
                        &mut rt as *mut sgx_status_t,
                        buf.as_slice().as_ptr() as *const sgx_platform_info_t,
                        1,
                        &mut update_info as *mut sgx_update_info_bit_t,
                    )
                };
                if res != sgx_status_t::SGX_SUCCESS {
                    return Err(Error::SGXError(
                        res,
                        "failed to ocall_get_update_info".to_string(),
                    ));
                }

                if rt != sgx_status_t::SGX_SUCCESS {
                    info!("rt={:?}", rt);
                    // Borrow of packed field is unsafe in future Rust releases
                    info!("update_info.pswUpdate: {}", update_info.pswUpdate as i32);
                    info!(
                        "update_info.csmeFwUpdate: {}",
                        update_info.csmeFwUpdate as i32
                    );
                    info!(
                        "update_info.ucodeUpdate: {}",
                        update_info.ucodeUpdate as i32
                    );
                    if !RT_ALLOWED_STATUS.contains(&rt) {
                        return Err(Error::SGXError(rt, "the status is not allowed".to_string()));
                    }
                }
            } else {
                info!("attestation report doesn't contain platformInfoBlob");
            }
        }
        _ => unreachable!("isv_enclave_quote_status must not be empty"),
    }

    Ok(quote)
}

use crate::errors::Error;
use crate::prelude::*;
use attestation_report::{AttestationVerificationReport, Quote};
use core::time::Duration;
use host_api::remote_attestation::get_report_attestation_status;
use lcp_types::Time;
use log::*;
use ocall_commands::{GetReportAttestationStatusInput, GetReportAttestationStatusResult};
use settings::RT_ALLOWED_STATUS;
use sgx_types::{sgx_platform_info_t, sgx_status_t};

pub fn validate_quote_status(avr: &AttestationVerificationReport) -> Result<Quote, Error> {
    // 1. Verify quote body
    let quote = avr.parse_quote().map_err(Error::attestation_report)?;

    // 2. Check quote's timestamp is within 24H
    let now = Time::now();
    info!(
        "Time: now={:?} quote_timestamp={:?}",
        now, quote.attestation_time
    );

    if now >= (quote.attestation_time + Duration::from_secs(60 * 60 * 24)).map_err(Error::time)? {
        return Err(Error::too_old_report_timestamp(now, quote.attestation_time));
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
                let mut platform_info = sgx_platform_info_t::default();
                platform_info.platform_info.copy_from_slice(buf.as_slice());

                let GetReportAttestationStatusResult { ret, update_info } =
                    get_report_attestation_status(GetReportAttestationStatusInput {
                        platform_blob: platform_info,
                        enclave_trusted: 1,
                    })
                    .map_err(Error::host_api)?;

                if ret != sgx_status_t::SGX_SUCCESS {
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
                    if !RT_ALLOWED_STATUS.contains(&ret) {
                        return Err(Error::sgx_error(
                            ret,
                            "the status is not allowed".to_string(),
                        ));
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

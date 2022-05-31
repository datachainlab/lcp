use chrono::{prelude::*, Duration};
use log::*;
use serde_json::Value;
use settings::RT_ALLOWED_STATUS;
use sgx_types::{sgx_platform_info_t, sgx_quote_t, sgx_status_t, sgx_update_info_bit_t};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::untrusted::time::SystemTimeEx;
use std::vec::Vec;

use super::ocalls;

pub fn verify_quote_status(attn_report: &[u8]) -> Result<sgx_quote_t, sgx_status_t> {
    let attn_report: Value = serde_json::from_slice(attn_report).unwrap();
    // 1. Check timestamp is within 24H
    if let Value::String(time) = &attn_report["timestamp"] {
        let time_fixed = time.clone() + "+0000";
        let ts = DateTime::parse_from_str(&time_fixed, "%Y-%m-%dT%H:%M:%S%.f%z")
            .unwrap()
            .timestamp();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(ts >= 0 && now >= 0);
        info!("Time: now={} ts={}", now, ts);
        if now - ts >= Duration::hours(24).num_seconds() {
            error!(
                "The timestamp of the report is too old: now={} ts={}",
                now, ts
            );
            return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
        }
    } else {
        error!("Failed to fetch timestamp from attestation report");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }

    if let Value::String(version) = &attn_report["version"] {
        if version != "4" {
            return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
        }
    }

    // 2. Verify quote status (mandatory field)
    if let Value::String(quote_status) = &attn_report["isvEnclaveQuoteStatus"] {
        match quote_status.as_ref() {
            "OK" => (),
            "GROUP_OUT_OF_DATE" | "GROUP_REVOKED" | "CONFIGURATION_NEEDED" => {
                // Verify platformInfoBlob for further info if status not OK
                // This is optional
                if let Value::String(pib) = &attn_report["platformInfoBlob"] {
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
                        info!("res={:?}", res);
                        return Err(res);
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
                            return Err(rt);
                        }
                    }
                } else {
                    error!("Failed to fetch platformInfoBlob from attestation report");
                    return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
                }
            }
            _ => return Err(sgx_status_t::SGX_ERROR_UNEXPECTED),
        }
    } else {
        error!("Failed to fetch isvEnclaveQuoteStatus from attestation report");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }

    // 3. Verify quote body
    match &attn_report["isvEnclaveQuoteBody"] {
        Value::String(quote_raw) => {
            let quote = base64::decode(&quote_raw).unwrap();

            let sgx_quote: sgx_quote_t = unsafe { ptr::read(quote.as_ptr() as *const _) };
            Ok(sgx_quote)
        }
        _ => {
            error!("Failed to fetch isvEnclaveQuoteBody from attestation report");
            Err(sgx_status_t::SGX_ERROR_UNEXPECTED)
        }
    }
}

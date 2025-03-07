use crate::errors::Error;
use crate::get_target_qe_info;
use crate::ias_utils::{get_quote, validate_qe_report, SGX_QUOTE_SIGN_TYPE};
use attestation_report::{IASAttestationVerificationReport, IASSignedReport, QEType};
use base64::{engine::general_purpose::STANDARD as Base64Std, Engine};
use crypto::Address;
use keymanager::EnclaveKeyManager;
use log::*;
use rsa::signature::{SignatureEncoding, Signer};

/// Run IAS RA simulation
pub fn run_ias_ra_simulation(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
    signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
    signing_cert: Vec<u8>,
) -> Result<IASSignedReport, Error> {
    let (target_info, _) = get_target_qe_info(QEType::QE)?;
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;
    if ek_info.qe_type != QEType::QE {
        return Err(Error::unexpected_qe_type(QEType::QE, ek_info.qe_type));
    }

    let (quote, qe_report) = get_quote(
        vec![],
        ek_info.report,
        SGX_QUOTE_SIGN_TYPE,
        Default::default(),
    )?;
    validate_qe_report(&target_info, &qe_report)?;
    let signed_report = create_simulate_avr(
        quote,
        advisory_ids,
        isv_enclave_quote_status,
        signing_key,
        signing_cert,
    )?;
    info!("IAS AVR: {:?}", signed_report.avr);
    info!(
        "report_data: {}",
        signed_report.get_avr()?.parse_quote()?.report_data()
    );
    key_manager
        .update_ra_quote(target_enclave_key, signed_report.clone().into())
        .map_err(|e| {
            Error::key_manager(
                format!("cannot save IAS Simulation AVR: {}", target_enclave_key),
                e,
            )
        })?;

    Ok(signed_report)
}

fn create_simulate_avr(
    quote: Vec<u8>,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
    signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
    signing_cert: Vec<u8>,
) -> Result<IASSignedReport, Error> {
    let now = chrono::Utc::now();
    // TODO more configurable via simulation command
    let avr = IASAttestationVerificationReport {
        id: "23856791181030202675484781740313693463".to_string(),
        // TODO refactoring
        timestamp: format!(
            "{}000",
            now.format("%Y-%m-%dT%H:%M:%S%.f%z")
                .to_string()
                .strip_suffix("+0000")
                .unwrap()
                .to_string()
        ),
        version: 4,
        advisory_url: "https://security-center.intel.com".to_string(),
        advisory_ids,
        isv_enclave_quote_status,
        platform_info_blob: None,
        isv_enclave_quote_body: Base64Std.encode(&quote.as_slice()[..432]),
        ..Default::default()
    };
    let avr_json = avr.to_canonical_json().unwrap();
    let signature = signing_key.sign(avr_json.as_bytes()).to_vec();
    Ok(IASSignedReport {
        avr: avr_json,
        signature,
        signing_cert,
    })
}

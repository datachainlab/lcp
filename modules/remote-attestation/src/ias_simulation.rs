use crate::errors::Error;
use crate::ias_utils::{get_quote, init_quote, validate_qe_report, SGX_QUOTE_SIGN_TYPE};
use attestation_report::{AttestationVerificationReport, EndorsedAttestationVerificationReport};
use base64::{engine::general_purpose::STANDARD as Base64Std, Engine};
use crypto::Address;
use ecall_commands::{CreateReportInput, CreateReportResponse};
use enclave_api::EnclaveCommandAPI;
use rsa::signature::{SignatureEncoding, Signer};
use store::transaction::CommitStore;

pub fn run_ias_ra_simulation<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: &E,
    target_enclave_key: Address,
    operator: Option<Address>,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
    signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
    signing_cert: Vec<u8>,
) -> Result<EndorsedAttestationVerificationReport, Error> {
    let (target_info, _) = init_quote()?;
    let CreateReportResponse { report } = enclave
        .create_report(CreateReportInput {
            target_info,
            target_enclave_key,
            operator,
        })
        .map_err(Error::enclave_api)?;

    let (quote, qe_report) = get_quote(vec![], report, SGX_QUOTE_SIGN_TYPE, Default::default())?;
    validate_qe_report(&target_info, &qe_report)?;
    create_simulate_avr(
        quote,
        advisory_ids,
        isv_enclave_quote_status,
        signing_key,
        signing_cert,
    )
}

fn create_simulate_avr(
    quote: Vec<u8>,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
    signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
    signing_cert: Vec<u8>,
) -> Result<EndorsedAttestationVerificationReport, Error> {
    let now = chrono::Utc::now();
    // TODO more configurable via simulation command
    let avr = AttestationVerificationReport {
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
    Ok(EndorsedAttestationVerificationReport {
        avr: avr_json,
        signature,
        signing_cert,
    })
}

use crate::attestation::REPORT_DATA_SIZE;
use crate::errors::Error;
use crate::prelude::*;
use attestation_report::AttestationVerificationReport;
use crypto::sgx::rand::fill_bytes;
use host_api::remote_attestation::{get_quote, init_quote};
use itertools::Itertools;
use log::*;
use ocall_commands::{GetQuoteInput, GetQuoteResult, InitQuoteResult};
use sgx_tcrypto::rsgx_sha256_slice;
use sgx_tse::{rsgx_create_report, rsgx_verify_report};
use sgx_types::{sgx_quote_nonce_t, sgx_quote_sign_type_t, sgx_report_data_t};

pub fn create_attestation_report(
    report_data: sgx_report_data_t,
    sign_type: sgx_quote_sign_type_t,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
) -> Result<AttestationVerificationReport, Error> {
    // Workflow:
    // (1) ocall to get the target_info structure and epid_group_id
    // (1.5) get sigrl
    // (2) call sgx_create_report with target_info+data, produce an sgx_report_t
    // (3) ocall to sgx_get_quote to generate (*mut sgx-quote_t, uint32_t)

    // (1) get target_info + epid_group_id

    let InitQuoteResult {
        target_info,
        epid_group_id,
    } = init_quote().map_err(Error::host_api)?;

    trace!("EPID group = {:?}", epid_group_id);

    // (2) Generate the report
    // Fill secp256k1 public key into report_data
    // this is given as a parameter

    let report = match rsgx_create_report(&target_info, &report_data) {
        Ok(r) => {
            trace!(
                "Report creation => success. Got MR_ENCLAVE {:?}",
                r.body.mr_enclave.m
            );
            r
        }
        Err(e) => {
            return Err(Error::sgx_error(e, "Report creation => failed".to_string()));
        }
    };

    let mut quote_nonce = sgx_quote_nonce_t { rand: [0; 16] };
    fill_bytes(&mut quote_nonce.rand)
        .map_err(|e| Error::sgx_error(e, "failed to fill_bytes".to_string()))?;
    trace!("Nonce generated successfully");

    // (3) Generate the quote
    // Args:
    //       1. sigrl: ptr + len
    //       2. report: ptr 432bytes
    //       3. linkable: u32, unlinkable=0, linkable=1
    //       4. spid: sgx_spid_t ptr 16bytes
    //       5. sgx_quote_nonce_t ptr 16bytes
    //       6. p_sig_rl + sigrl size ( same to sigrl)
    //       7. [out]p_qe_report need further check
    //       8. [out]p_quote
    //       9. quote_size

    let GetQuoteResult { qe_report, quote } = get_quote(GetQuoteInput {
        sigrl: vec![],
        report,
        quote_type: sign_type,
        spid: Default::default(),
        nonce: quote_nonce,
    })
    .map_err(Error::host_api)?;

    // Added 09-28-2018
    // Perform a check on qe_report to verify if the qe_report is valid
    match rsgx_verify_report(&qe_report) {
        Ok(()) => trace!("rsgx_verify_report passed!"),
        Err(e) => {
            return Err(Error::sgx_error(e, "rsgx_verify_report failed".to_string()));
        }
    }

    // Check if the qe_report is produced on the same platform
    if target_info.mr_enclave.m != qe_report.body.mr_enclave.m
        || target_info.attributes.flags != qe_report.body.attributes.flags
        || target_info.attributes.xfrm != qe_report.body.attributes.xfrm
    {
        return Err(Error::unexpected_report(
            "qe_report does not match current target_info!".to_string(),
        ));
    }

    trace!("QE report check passed");

    // Check qe_report to defend against replay attack
    // The purpose of p_qe_report is for the ISV enclave to confirm the QUOTE
    // it received is not modified by the untrusted SW stack, and not a replay.
    // The implementation in QE is to generate a REPORT targeting the ISV
    // enclave (target info from p_report) , with the lower 32Bytes in
    // report.data = SHA256(p_nonce||p_quote). The ISV enclave can verify the
    // p_qe_report and report.data to confirm the QUOTE has not be modified and
    // is not a replay. It is optional.

    let mut rhs_vec: Vec<u8> = quote_nonce.rand.to_vec();
    rhs_vec.extend(&quote);
    let rhs_hash = rsgx_sha256_slice(&rhs_vec[..]).unwrap();
    let lhs_hash = &qe_report.body.report_data.d[..REPORT_DATA_SIZE];

    trace!("Report rhs hash = {:02X}", rhs_hash.iter().format(""));
    trace!("Report lhs hash = {:02X}", lhs_hash.iter().format(""));

    if rhs_hash != lhs_hash {
        return Err(Error::unexpected_quote(
            format!("Quote is tampered!: {:?} != {:?}", rhs_hash, lhs_hash).to_string(),
        ));
    }

    create_simulate_avr(quote, advisory_ids, isv_enclave_quote_status)
}

fn create_simulate_avr(
    quote: Vec<u8>,
    advisory_ids: Vec<String>,
    isv_enclave_quote_status: String,
) -> Result<AttestationVerificationReport, Error> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use sgx_tstd::time::SystemTime;

    let now = {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let now = NaiveDateTime::from_timestamp_millis(now.as_millis() as i64).unwrap();
        DateTime::<Utc>::from_utc(now, Utc)
    };
    // TODO more configurable via simulation command
    Ok(AttestationVerificationReport {
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
        isv_enclave_quote_body: base64::encode(&quote.as_slice()[..432]),
        ..Default::default()
    })
}

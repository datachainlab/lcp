use crate::errors::Error;
use attestation_report::DCAPQuote;
use crypto::Address;
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use sgx_types::{sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_quote3_error_t, sgx_report_t};

pub fn run_dcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
) -> Result<(), Error> {
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;
    let quote = rsgx_qe_get_quote(&ek_info.report).unwrap();
    println!("Successfully get the quote: {:?}", quote);
    let current_time = Time::now();
    // libqvl_verify_quote(&quote, current_time)?;
    key_manager
        .save_verifiable_quote(
            target_enclave_key,
            DCAPQuote::new(quote, current_time).into(),
        )
        .map_err(|e| {
            Error::key_manager(format!("cannot save DCAP AVR: {}", target_enclave_key), e)
        })?;
    Ok(())
}

fn rsgx_qe_get_quote(app_report: &sgx_report_t) -> Result<Vec<u8>, sgx_quote3_error_t> {
    let mut quote_size = 0;
    unsafe {
        match sgx_qe_get_quote_size(&mut quote_size) {
            sgx_quote3_error_t::SGX_QL_SUCCESS => {
                let mut quote = vec![0u8; quote_size as usize];
                match sgx_qe_get_quote(app_report, quote_size, quote.as_mut_ptr()) {
                    sgx_quote3_error_t::SGX_QL_SUCCESS => Ok(quote),
                    err => Err(err),
                }
            }
            err => Err(err),
        }
    }
}

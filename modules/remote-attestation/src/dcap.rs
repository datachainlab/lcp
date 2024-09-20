use crate::errors::Error;
use crypto::Address;
use ecall_commands::{CreateReportInput, CreateReportResponse};
use enclave_api::EnclaveCommandAPI;
use sgx_types::{
    sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_qe_get_target_info, sgx_quote3_error_t,
    sgx_report_t, sgx_target_info_t, tee_get_supplemental_data_version_and_size,
};
use store::transaction::CommitStore;

pub fn run_dcap_ra<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: &E,
    target_enclave_key: Address,
    operator: Option<Address>,
) -> Result<(), Error> {
    let mut target_info: sgx_target_info_t = Default::default();
    let result = unsafe { sgx_qe_get_target_info(&mut target_info) };
    if result != sgx_quote3_error_t::SGX_QL_SUCCESS {
        panic!("Failed to get the target_info");
    }
    println!("Successfully get the target_info");
    let CreateReportResponse { report } = enclave
        .create_report(CreateReportInput {
            target_info,
            target_enclave_key,
            operator,
        })
        .map_err(Error::enclave_api)?;

    println!("Successfully create the report: {:?}", report);

    let quote = rsgx_qe_get_quote(&report).unwrap();
    println!("Successfully get the quote: {:?}", quote);

    let res = rsgx_tee_get_supplemental_data_version_and_size(&quote);
    println!(
        "Successfully get the supplemental data version and size: {:?}",
        res
    );

    Ok(())
}

// https://github.com/intel/SGXDataCenterAttestationPrimitives/blob/eff36080bc3b8186745796b1ff9f067036e21a3a/QuoteGeneration/quote_wrapper/sgx-dcap-ql-rs/src/lib.rs
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

fn rsgx_tee_get_supplemental_data_version_and_size(
    quote: &[u8],
) -> Result<(u32, u32), sgx_quote3_error_t> {
    let mut version = 0u32;
    let mut data_size = 0u32;

    match unsafe {
        tee_get_supplemental_data_version_and_size(
            quote.as_ptr(),
            quote.len() as u32,
            &mut version,
            &mut data_size,
        )
    } {
        sgx_quote3_error_t::SGX_QL_SUCCESS => Ok((version, data_size)),
        error_code => Err(error_code),
    }
}

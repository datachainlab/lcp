use crate::errors::Error;
use core::slice;
use crypto::Address;
use ecall_commands::{CreateReportInput, CreateReportResponse};
use enclave_api::EnclaveCommandAPI;
use intel_tee_quote_verification_sys::sgx_ql_qve_collateral_t;
use sgx_types::{
    sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_qe_get_target_info, sgx_quote3_error_t,
    sgx_report_t, sgx_target_info_t, tee_get_supplemental_data_version_and_size,
    tee_qv_free_collateral, tee_qv_get_collateral,
};
use std::os::raw::c_char;
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

    let collateral = rsgx_tee_qv_get_collateral(&quote).unwrap();
    println!("Successfully get the collateral: {:?}", collateral);

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

/// Get quote verification collateral.
///
/// # Param
/// - **quote**\
/// SGX/TDX Quote, presented as u8 vector.
///
/// # Return
/// Result type of quote_collecteral.
///
/// - **quote_collateral**\
/// This is the Quote Certification Collateral retrieved based on Quote.
///
/// Status code of the operation, one of:
/// - *SGX_QL_ERROR_INVALID_PARAMETER*
/// - *SGX_QL_PLATFORM_LIB_UNAVAILABLE*
/// - *SGX_QL_PCK_CERT_CHAIN_ERROR*
/// - *SGX_QL_PCK_CERT_UNSUPPORTED_FORMAT*
/// - *SGX_QL_QUOTE_FORMAT_UNSUPPORTED*
/// - *SGX_QL_OUT_OF_MEMORY*
/// - *SGX_QL_NO_QUOTE_COLLATERAL_DATA*
/// - *SGX_QL_ERROR_UNEXPECTED*
///
pub fn rsgx_tee_qv_get_collateral(quote: &[u8]) -> Result<QuoteCollateral, sgx_quote3_error_t> {
    let mut buf = core::ptr::null_mut();
    let mut buf_len = 0u32;

    match unsafe {
        tee_qv_get_collateral(quote.as_ptr(), quote.len() as u32, &mut buf, &mut buf_len)
    } {
        sgx_quote3_error_t::SGX_QL_SUCCESS => {
            assert!(!buf.is_null());
            assert!(buf_len > 0);
            assert_eq!(
                buf.align_offset(core::mem::align_of::<sgx_ql_qve_collateral_t>()),
                0
            );

            let collateral =
                QuoteCollateral::from(unsafe { *(buf as *const sgx_ql_qve_collateral_t) });

            match unsafe { tee_qv_free_collateral(buf) } {
                sgx_quote3_error_t::SGX_QL_SUCCESS => Ok(collateral),
                error_code => Err(error_code),
            }
        }
        error_code => Err(error_code),
    }
}

/// Quote Certification Collateral Structure
///
#[derive(Debug, Clone)]
pub struct QuoteCollateral {
    pub major_version: u16,
    pub minor_version: u16,
    pub tee_type: u32,
    pub pck_crl_issuer_chain: Vec<c_char>,
    pub root_ca_crl: Vec<c_char>,
    pub pck_crl: Vec<c_char>,
    pub tcb_info_issuer_chain: Vec<c_char>,
    pub tcb_info: Vec<c_char>,
    pub qe_identity_issuer_chain: Vec<c_char>,
    pub qe_identity: Vec<c_char>,
}

impl From<sgx_ql_qve_collateral_t> for QuoteCollateral {
    fn from(collateral: sgx_ql_qve_collateral_t) -> Self {
        fn raw_ptr_to_vec(data: *mut c_char, len: u32) -> Vec<c_char> {
            assert!(!data.is_null());
            unsafe { slice::from_raw_parts(data, len as _) }.to_vec()
        }

        QuoteCollateral {
            major_version: unsafe { collateral.__bindgen_anon_1.__bindgen_anon_1.major_version },
            minor_version: unsafe { collateral.__bindgen_anon_1.__bindgen_anon_1.minor_version },
            tee_type: collateral.tee_type,
            pck_crl_issuer_chain: raw_ptr_to_vec(
                collateral.pck_crl_issuer_chain,
                collateral.pck_crl_issuer_chain_size,
            ),
            root_ca_crl: raw_ptr_to_vec(collateral.root_ca_crl, collateral.root_ca_crl_size),
            pck_crl: raw_ptr_to_vec(collateral.pck_crl, collateral.pck_crl_size),
            tcb_info_issuer_chain: raw_ptr_to_vec(
                collateral.tcb_info_issuer_chain,
                collateral.tcb_info_issuer_chain_size,
            ),
            tcb_info: raw_ptr_to_vec(collateral.tcb_info, collateral.tcb_info_size),
            qe_identity_issuer_chain: raw_ptr_to_vec(
                collateral.qe_identity_issuer_chain,
                collateral.qe_identity_issuer_chain_size,
            ),
            qe_identity: raw_ptr_to_vec(collateral.qe_identity, collateral.qe_identity_size),
        }
    }
}

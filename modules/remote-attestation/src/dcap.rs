use crate::errors::Error;
use core::{ptr, slice};
use crypto::Address;
use ecall_commands::{CreateReportInput, CreateReportResponse};
use enclave_api::EnclaveCommandAPI;
use intel_tee_quote_verification_sys::{
    _sgx_ql_qve_collateral_t__bindgen_ty_1, _sgx_ql_qve_collateral_t__bindgen_ty_1__bindgen_ty_1,
    sgx_ql_qv_supplemental_t, sgx_ql_qve_collateral_t,
};
use sgx_types::{
    sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_qe_get_target_info, sgx_ql_qe_report_info_t,
    sgx_ql_qv_result_t, sgx_quote3_error_t, sgx_report_t, sgx_target_info_t,
    tee_get_supplemental_data_version_and_size, tee_qv_free_collateral, tee_qv_get_collateral,
    tee_supp_data_descriptor_t, tee_verify_quote,
};
use std::{
    marker::PhantomData,
    ops::Deref,
    os::raw::c_char,
    time::{Duration, SystemTime},
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

    let collateral = rsgx_tee_qv_get_collateral(&quote);
    println!("Successfully get the collateral: {:?}", collateral);

    // set current time. This is only for sample purposes, in production mode a trusted time should be used.
    //
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;

    let mut supp_data: sgx_ql_qv_supplemental_t = Default::default();
    let mut supp_data_desc = tee_supp_data_descriptor_t {
        major_version: 0,
        data_size: 0,
        p_data: &mut supp_data as *mut sgx_ql_qv_supplemental_t as *mut u8,
    };

    let p_supplemental_data = match supp_data_desc.data_size {
        0 => None,
        _ => Some(&mut supp_data_desc),
    };

    // call DCAP quote verify library for quote verification
    // here you can choose 'trusted' or 'untrusted' quote verification by specifying parameter '&qve_report_info'
    // if '&qve_report_info' is NOT NULL, this API will call Intel QvE to verify quote
    // if '&qve_report_info' is NULL, this API will call 'untrusted quote verify lib' to verify quote, this mode doesn't rely on SGX capable system, but the results can not be cryptographically authenticated
    let (collateral_expiration_status, quote_verification_result) = match rsgx_tee_verify_quote(
        &quote,
        collateral.ok().as_ref(),
        current_time,
        None,
        p_supplemental_data,
    ) {
        Ok((colla_exp_stat, qv_result)) => {
            println!(
                "\tInfo: App: tee_verify_quote successfully returned: {} {:?}",
                colla_exp_stat, qv_result
            );
            (colla_exp_stat, qv_result)
        }
        Err(e) => panic!("\tError: App: tee_verify_quote failed: {:#04x}", e as u32),
    };

    // check verification result
    //
    match quote_verification_result {
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK => {
            // check verification collateral expiration status
            // this value should be considered in your own attestation/verification policy
            //
            if collateral_expiration_status == 0 {
                println!("\tInfo: App: Verification completed successfully.");
            } else {
                println!("\tWarning: App: Verification completed, but collateral is out of date based on 'expiration_check_date' you provided.");
            }
        }
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_SW_HARDENING_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED => {
            println!(
                "\tWarning: App: Verification completed with Non-terminal result: {:x}",
                quote_verification_result as u32
            );
        }
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_INVALID_SIGNATURE
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_REVOKED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_UNSPECIFIED
        | _ => {
            println!(
                "\tError: App: Verification completed with Terminal result: {:x}",
                quote_verification_result as u32
            );
        }
    }

    // check supplemental data if necessary
    //
    if supp_data_desc.data_size > 0 {
        // you can check supplemental data based on your own attestation/verification policy
        // here we only print supplemental data version for demo usage
        //
        let version_s = unsafe { supp_data.__bindgen_anon_1.__bindgen_anon_1 };
        println!(
            "\tInfo: Supplemental data Major Version: {}",
            version_s.major_version
        );
        println!(
            "\tInfo: Supplemental data Minor Version: {}",
            version_s.minor_version
        );

        // print SA list if exist, SA list is supported from version 3.1
        //
        if unsafe { supp_data.__bindgen_anon_1.version } > 3 {
            let sa_list = unsafe { std::ffi::CStr::from_ptr(supp_data.sa_list.as_ptr()) };
            if sa_list.to_bytes().len() > 0 {
                println!("\tInfo: Advisory ID: {}", sa_list.to_str().unwrap());
            }
        }
    }

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

struct PhantomCollateral<'a> {
    inner: sgx_ql_qve_collateral_t,
    phantom: PhantomData<&'a ()>,
}

impl<'a> From<&'a QuoteCollateral> for PhantomCollateral<'a> {
    fn from(collateral: &'a QuoteCollateral) -> Self {
        PhantomCollateral {
            inner: sgx_ql_qve_collateral_t {
                __bindgen_anon_1: _sgx_ql_qve_collateral_t__bindgen_ty_1 {
                    __bindgen_anon_1: _sgx_ql_qve_collateral_t__bindgen_ty_1__bindgen_ty_1 {
                        major_version: collateral.major_version,
                        minor_version: collateral.minor_version,
                    },
                },
                tee_type: collateral.tee_type,
                pck_crl_issuer_chain: collateral.pck_crl_issuer_chain.as_ptr() as _,
                pck_crl_issuer_chain_size: collateral.pck_crl_issuer_chain.len() as _,
                root_ca_crl: collateral.root_ca_crl.as_ptr() as _,
                root_ca_crl_size: collateral.root_ca_crl.len() as _,
                pck_crl: collateral.pck_crl.as_ptr() as _,
                pck_crl_size: collateral.pck_crl.len() as _,
                tcb_info_issuer_chain: collateral.tcb_info_issuer_chain.as_ptr() as _,
                tcb_info_issuer_chain_size: collateral.tcb_info_issuer_chain.len() as _,
                tcb_info: collateral.tcb_info.as_ptr() as _,
                tcb_info_size: collateral.tcb_info.len() as _,
                qe_identity_issuer_chain: collateral.qe_identity_issuer_chain.as_ptr() as _,
                qe_identity_issuer_chain_size: collateral.qe_identity_issuer_chain.len() as _,
                qe_identity: collateral.qe_identity.as_ptr() as _,
                qe_identity_size: collateral.qe_identity.len() as _,
            },
            phantom: PhantomData,
        }
    }
}

impl<'a> Deref for PhantomCollateral<'a> {
    type Target = sgx_ql_qve_collateral_t;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Perform quote verification for SGX and TDX.\
/// This API works the same as the old one, but takes a new parameter to describe the supplemental data (supp_data_descriptor).
///
/// # Param
/// - **quote**\
/// SGX/TDX Quote, presented as u8 vector.
/// - **quote_collateral**\
/// Quote Certification Collateral provided by the caller.
/// - **expiration_check_date**\
/// This is the date that the QvE will use to determine if any of the inputted collateral have expired.
/// - **qve_report_info**\
/// This parameter can be used in 2 ways.\
///     - If qve_report_info is NOT None, the API will use Intel QvE to perform quote verification, and QvE will generate a report using the target_info in sgx_ql_qe_report_info_t structure.\
///     - if qve_report_info is None, the API will use QVL library to perform quote verification, note that the results can not be cryptographically authenticated in this mode.
/// - **supp_datal_descriptor**\
/// *tee_supp_data_descriptor_t* structure.\
/// You can specify the major version of supplemental data by setting supp_datal_descriptor.major_version.\
/// If supp_datal_descriptor is None, no supplemental data is returned.\
/// If supp_datal_descriptor.major_version == 0, then return the latest version of the *sgx_ql_qv_supplemental_t* structure.\
/// If supp_datal_descriptor.major_version <= latest supported version, return the latest minor version associated with that major version.\
/// If supp_datal_descriptor.major_version > latest supported version, return an error *SGX_QL_SUPPLEMENTAL_DATA_VERSION_NOT_SUPPORTED*.
///
/// # Return
/// Result type of (collateral_expiration_status, verification_result).
///
/// Status code of the operation, one of:
/// - *SGX_QL_ERROR_INVALID_PARAMETER*
/// - *SGX_QL_QUOTE_FORMAT_UNSUPPORTED*
/// - *SGX_QL_QUOTE_CERTIFICATION_DATA_UNSUPPORTED*
/// - *SGX_QL_UNABLE_TO_GENERATE_REPORT*
/// - *SGX_QL_CRL_UNSUPPORTED_FORMAT*
/// - *SGX_QL_ERROR_UNEXPECTED*
///
pub fn rsgx_tee_verify_quote(
    quote: &[u8],
    quote_collateral: Option<&QuoteCollateral>,
    expiration_check_date: i64,
    qve_report_info: Option<&mut sgx_ql_qe_report_info_t>,
    supp_data_descriptor: Option<&mut tee_supp_data_descriptor_t>,
) -> Result<(u32, sgx_ql_qv_result_t), sgx_quote3_error_t> {
    let mut collateral_expiration_status = 1u32;
    let mut quote_verification_result = sgx_ql_qv_result_t::SGX_QL_QV_RESULT_UNSPECIFIED;

    let quote_collateral = quote_collateral.map(PhantomCollateral::from);
    let p_quote_collateral = quote_collateral.as_deref().map_or(ptr::null(), |p| p);
    let p_qve_report_info = qve_report_info.map_or(ptr::null_mut(), |p| p);
    let p_supp_data_descriptor = supp_data_descriptor.map_or(ptr::null_mut(), |p| p);

    match unsafe {
        tee_verify_quote(
            quote.as_ptr(),
            quote.len() as u32,
            p_quote_collateral as _,
            expiration_check_date,
            &mut collateral_expiration_status,
            &mut quote_verification_result,
            p_qve_report_info,
            p_supp_data_descriptor,
        )
    } {
        sgx_quote3_error_t::SGX_QL_SUCCESS => {
            Ok((collateral_expiration_status, quote_verification_result))
        }
        error_code => Err(error_code),
    }
}

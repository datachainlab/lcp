use crate::errors::Error;
use attestation_report::DCAPQuote;
use core::{ptr, slice};
use crypto::Address;
use enclave_api::EnclaveCommandAPI;
use intel_tee_quote_verification_sys::{
    _sgx_ql_qve_collateral_t__bindgen_ty_1, _sgx_ql_qve_collateral_t__bindgen_ty_1__bindgen_ty_1,
    quote3_error_t, sgx_ql_qv_supplemental_t, sgx_ql_qve_collateral_t, tee_get_fmspc_from_quote,
};
use lcp_types::Time;
use log::*;
use sgx_types::{
    sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_ql_qe_report_info_t, sgx_ql_qv_result_t,
    sgx_quote3_error_t, sgx_report_t, tee_get_supplemental_data_version_and_size,
    tee_qv_free_collateral, tee_qv_get_collateral, tee_supp_data_descriptor_t, tee_verify_quote,
};
use std::{marker::PhantomData, ops::Deref, os::raw::c_char};
use store::transaction::CommitStore;

pub fn run_dcap_ra<E: EnclaveCommandAPI<S>, S: CommitStore>(
    enclave: &E,
    target_enclave_key: Address,
) -> Result<(), Error> {
    let ek_info = enclave
        .get_key_manager()
        .load(target_enclave_key)
        .map_err(|e| {
            Error::key_manager(
                format!("cannot load enclave key: {}", target_enclave_key),
                e,
            )
        })?;
    let quote = rsgx_qe_get_quote(&ek_info.report).unwrap();
    println!("Successfully get the quote: {:?}", quote);
    let current_time = Time::now();
    verify_quote(&quote, current_time)?;
    enclave
        .get_key_manager()
        .save_verifiable_quote(
            target_enclave_key,
            DCAPQuote::new(quote, current_time).into(),
        )
        .map_err(|e| {
            Error::key_manager(format!("cannot save DCAP AVR: {}", target_enclave_key), e)
        })?;
    Ok(())
}

fn verify_quote(quote: &[u8], current_time: Time) -> Result<(), Error> {
    let mut fmspc: [u8; 6] = Default::default();
    match unsafe {
        tee_get_fmspc_from_quote(
            quote.as_ptr(),
            quote.len() as u32,
            fmspc.as_mut_ptr(),
            fmspc.len() as u32,
        )
    } {
        quote3_error_t::SGX_QL_SUCCESS => {
            info!("successfully get the fmspc: 0x{}", hex::encode(fmspc));
        }
        error_code => {
            panic!("failed to get the fmspc: {:?}", error_code);
        }
    }

    let mut supp_data: sgx_ql_qv_supplemental_t = Default::default();
    let mut supp_data_desc = tee_supp_data_descriptor_t {
        major_version: 0,
        data_size: 0,
        p_data: &mut supp_data as *mut sgx_ql_qv_supplemental_t as *mut u8,
    };

    match rsgx_tee_get_supplemental_data_version_and_size(&quote) {
        Ok((supp_ver, supp_size)) => {
            if supp_size == core::mem::size_of::<sgx_ql_qv_supplemental_t>() as u32 {
                info!("tee_get_quote_supplemental_data_version_and_size successfully returned");
                info!(
                    "latest supplemental data major version: {}, minor version: {}, size: {}",
                    u16::from_be_bytes(supp_ver.to_be_bytes()[..2].try_into().unwrap()),
                    u16::from_be_bytes(supp_ver.to_be_bytes()[2..].try_into().unwrap()),
                    supp_size,
                );
                supp_data_desc.data_size = supp_size;
            } else {
                warn!("Quote supplemental data size is different between DCAP QVL and QvE, please make sure you installed DCAP QVL and QvE from same release")
            }
        }
        Err(e) => {
            panic!(
                "Failed to get the supplemental data version and size: {:?}",
                e
            );
        }
    }

    let collateral = rsgx_tee_qv_get_collateral(&quote);
    info!("successfully get the collateral: {:?}", collateral);

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
        current_time.unix_timestamp(),
        None,
        p_supplemental_data,
    ) {
        Ok((colla_exp_stat, qv_result)) => {
            info!(
                "tee_verify_quote successfully returned: {} {:?}",
                colla_exp_stat, qv_result
            );
            (colla_exp_stat, qv_result)
        }
        Err(e) => panic!("tee_verify_quote failed: {:#04x}", e as u32),
    };

    // check verification result
    //
    match quote_verification_result {
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK => {
            // check verification collateral expiration status
            // this value should be considered in your own attestation/verification policy
            //
            if collateral_expiration_status == 0 {
                info!("verification completed successfully");
            } else {
                info!("verification completed, but collateral is out of date based on 'expiration_check_date' you provided");
            }
        }
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_SW_HARDENING_NEEDED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED => {
            info!(
                "verification completed with Non-terminal result: {:x}",
                quote_verification_result as u32
            );
        }
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_INVALID_SIGNATURE
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_REVOKED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_UNSPECIFIED
        | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_MAX => {
            info!(
                "verification completed with Terminal result: {:x}",
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
        info!(
            "supplemental data Major Version: {}",
            version_s.major_version
        );
        info!(
            "supplemental data Minor Version: {}",
            version_s.minor_version
        );

        // print SA list if exist, SA list is supported from version 3.1
        //
        if unsafe { supp_data.__bindgen_anon_1.version } > 3 {
            let sa_list = unsafe { std::ffi::CStr::from_ptr(supp_data.sa_list.as_ptr()) };
            if !sa_list.to_bytes().is_empty() {
                info!("Advisory ID: {}", sa_list.to_str().unwrap());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_quote() {
        let quote = hex::decode("03000200000000000a000f00939a7233f79c4ca9940a0db3957f0607b5fe5d7f613d2d40b066b320879bd14d0000000015150b07ff800e000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000700000000000000ea4047329d65711f63993b9397245d44bafaee5e7d56a4906e7738f7cf697d7d000000000000000000000000000000000000000000000000000000000000000083d719e77deaca1470f6baf62a4d774303c899db69020f9c70ee1dfc08c7ce9e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000130fd34a0e3139da00ce0a9b845cb3ccf039b1bf700000000000000000000000000000000000000000000000000000000000000000000000000000000000000441000004bc221614d15399401379d302d1b8eae3a287957eac2497360c07d2fb3b237b9a2372e52dcdbe26560b1060274ebe0fefba61f1694b780b9c6e5dedc68e0e3624b1526520dd11db5efc9504fa42d048e37ba38c90c8873e7c62f72e86794797bcf8586b9e5c10d0866a95331548da898ae0adf78e428128324151ee558cfc71215150b07ff800e00000000000000000000000000000000000000000000000000000000000000000000000000000000001500000000000000070000000000000096b347a64e5a045e27369c26e6dcda51fd7c850e9b3a3a79e718f43261dee1e400000000000000000000000000000000000000000000000000000000000000008c4f5775d796503e96137f77c68a829a0056ac8ded70140b081b094490c57bff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017b0dc79c3dc5ff39b3f67346eef41f1ecd63e0a5259a9102eaace1f0aca06ec0000000000000000000000000000000000000000000000000000000000000000e6516d69dd50710c9d1eed1281bd9bb9bac590c23712a15d5cd648032d0524bc467ca02deb6a1bfb638e0df697a388419b18ac2d96b0cfca7dc9a5e6491e4eac2000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0500dc0d00002d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949456a6a4343424453674177494241674956414a34674a3835554b6b7a613873504a4847676e4f4b6d5451426e754d416f4743437147534d343942414d430a4d484578497a416842674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b0a4442464a626e526c6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e560a4241674d416b4e424d517377435159445651514745774a56557a4165467730794e4441354d5467784d4451354d6a56614677307a4d5441354d5467784d4451350a4d6a56614d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675132567964476c6d61574e6864475578476a415942674e560a42416f4d45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b470a413155454341774351304578437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741450a516a537877644d662b2b3578645553717478343769335952633970504a475434304642774e306e5335557a43314233524b63544875514c3135796b357a4c766c0a5535707a7563552f2b6d674a4e6f55774b6e784942364f434171677767674b6b4d42384741315564497751594d426141464e446f71747031312f6b75535265590a504873555a644456386c6c4e4d477747413155644877526c4d474d77596142666f463247573268306448427a4f693876595842704c6e527964584e305a57527a0a5a584a3261574e6c63793570626e526c6243356a62323076633264344c324e6c636e52705a6d6c6a5958527062323476646a517663474e7259334a7350324e680a5058427962324e6c63334e7663695a6c626d4e765a476c755a7a316b5a584977485159445652304f42425945464f7632356e4f67634c754f693644424b3037470a4d4f5161315a53494d41344741315564447745422f775145417749477744414d42674e5648524d4241663845416a41414d4949423141594a4b6f5a496876684e0a415130424249494278544343416345774867594b4b6f5a496876684e41513042415151514459697469663748386e4277566732482b38504f476a4343415751470a43697147534962345451454e41514977676746554d42414743797147534962345451454e41514942416745564d42414743797147534962345451454e415149430a416745564d42414743797147534962345451454e41514944416745434d42414743797147534962345451454e41514945416745454d42414743797147534962340a5451454e41514946416745424d42454743797147534962345451454e41514947416749416744415142677371686b69472b4530424451454342774942446a41510a42677371686b69472b45304244514543434149424144415142677371686b69472b45304244514543435149424144415142677371686b69472b453042445145430a436749424144415142677371686b69472b45304244514543437749424144415142677371686b69472b45304244514543444149424144415142677371686b69470a2b45304244514543445149424144415142677371686b69472b45304244514543446749424144415142677371686b69472b4530424451454344774942414441510a42677371686b69472b45304244514543454149424144415142677371686b69472b45304244514543455149424454416642677371686b69472b453042445145430a4567515146525543424147414467414141414141414141414144415142676f71686b69472b45304244514544424149414144415542676f71686b69472b4530420a44514545424159416b473756414141774477594b4b6f5a496876684e4151304242516f424144414b42676771686b6a4f5051514441674e4941444246416941450a4d50754f5455774e32794a7a6e30635362614a5654314e576d6e786d374a334d366a67626a424c523341496841496e354d5442363133744c6a33386a4b7432330a6d7a545743764b6735324d7941594c5578696632396a506a0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436d444343416a36674177494241674956414e446f71747031312f6b7553526559504873555a644456386c6c4e4d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484578497a41680a42674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b4442464a626e526c0a6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e420a4d517377435159445651514745774a56557a425a4d424d4742797147534d34394167454743437147534d34394177454841304941424c39712b4e4d7032494f670a74646c31626b2f75575a352b5447516d38614369387a373866732b664b435133642b75447a586e56544154325a68444369667949754a77764e33774e427039690a484253534d4a4d4a72424f6a6762737767626777487759445652306a42426777466f4155496d554d316c71644e496e7a6737535655723951477a6b6e427177770a556759445652306642457377535442486f45576751345a426148523063484d364c79396a5a584a3061575a70593246305a584d7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253394a626e526c62464e4857464a76623352445153356b5a584977485159445652304f42425945464e446f0a71747031312f6b7553526559504873555a644456386c6c4e4d41344741315564447745422f77514541774942426a415342674e5648524d4241663845434441470a4151482f416745414d416f4743437147534d343942414d43413067414d4555434951434a6754627456714f795a316d336a716941584d365159613672357357530a34792f4737793875494a4778647749675271507642534b7a7a516167424c517135733541373070646f6961524a387a2f3075447a344e675639316b3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap();
        let res = verify_quote(&quote, Time::from_unix_timestamp(1727956804, 0).unwrap());
        assert!(res.is_ok(), "Failed to verify quote: {:?}", res);
    }
}

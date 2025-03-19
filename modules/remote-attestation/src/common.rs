use crate::errors::Error;
use attestation_report::QEType;
use sgx_types::{
    sgx_epid_group_id_t, sgx_init_quote, sgx_qe_get_target_info, sgx_quote3_error_t, sgx_status_t,
    sgx_target_info_t,
};

/// Get target QE info
///
/// # Arguments
/// - `target_qe_type` - QE type(QE, QE3, QE3SIM)
///
/// # Returns
/// - `sgx_target_info_t` - Target info
/// - `sgx_epid_group_id_t` - If `target_qe_type` is QE, return epid group id. Otherwise, return default value
pub fn get_target_qe_info(
    target_qe_type: QEType,
) -> Result<(sgx_target_info_t, sgx_epid_group_id_t), Error> {
    let mut target_info = sgx_target_info_t::default();
    match target_qe_type {
        QEType::QE => {
            let mut epid_group_id = sgx_epid_group_id_t::default();
            match unsafe { sgx_init_quote(&mut target_info, &mut epid_group_id) } {
                sgx_status_t::SGX_SUCCESS => Ok((target_info, epid_group_id)),
                s => Err(Error::sgx_error(s, "failed to sgx_init_quote".into())),
            }
        }
        QEType::QE3 => match unsafe { sgx_qe_get_target_info(&mut target_info) } {
            sgx_quote3_error_t::SGX_QL_SUCCESS => Ok((target_info, sgx_epid_group_id_t::default())),
            s => Err(Error::sgx_qe3_error(
                s,
                "failed to sgx_qe_get_target_info".into(),
            )),
        },
        QEType::QE3SIM => Ok((target_info, sgx_epid_group_id_t::default())),
    }
}

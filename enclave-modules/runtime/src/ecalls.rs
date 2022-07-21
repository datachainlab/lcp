use crate::get_store;
use crate::light_client::GlobalLightClientRegistry;
use crypto::KeyManager;
use enclave_commands::EnclaveCommand;
use enclave_utils::validate_const_ptr;
use handler::router::dispatch;
use sgx_types::sgx_status_t;
use std::slice;

#[no_mangle]
pub unsafe extern "C" fn ecall_execute_command(
    command: *const u8,
    command_len: u32,
    output_buf: *mut u8,
    output_buf_maxlen: u32,
    output_len: &mut u32,
) -> sgx_status_t {
    validate_const_ptr!(
        command,
        command_len as usize,
        sgx_status_t::SGX_ERROR_UNEXPECTED
    );
    // TODO add error handling instead of unwrap

    let cmd: EnclaveCommand =
        bincode::deserialize(slice::from_raw_parts(command, command_len as usize)).unwrap();

    let km = KeyManager::new(cmd.params.home.clone());
    let result =
        dispatch::<_, GlobalLightClientRegistry>(km.get_enclave_key(), &mut get_store(), cmd)
            .unwrap();
    let res = bincode::serialize(&result).unwrap();
    assert!(
        output_buf_maxlen as usize >= res.len(),
        "{} >= {}",
        output_buf_maxlen as usize,
        res.len()
    );
    std::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len());
    *output_len = res.len() as u32;

    sgx_status_t::SGX_SUCCESS
}

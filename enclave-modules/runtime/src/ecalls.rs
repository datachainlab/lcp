use crate::get_store;
use crate::key_manager::KEY_MANAGER;
use crate::light_client::GlobalLightClientRegistry;
use enclave_commands::{Command, CommandResult, EnclaveManageResult};
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

    let cmd: Command =
        bincode::deserialize(slice::from_raw_parts(command, command_len as usize)).unwrap();

    let mut km = KEY_MANAGER.write().unwrap();
    let result =
        dispatch::<_, GlobalLightClientRegistry>(km.get_enclave_key(), get_store(), cmd).unwrap();
    // if InitEnclave is succeeded, load the generated key into the key manager
    if let CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(_)) = result {
        km.load_enclave_key().unwrap();
    }
    let res = bincode::serialize(&result).unwrap();
    assert!(output_buf_maxlen as usize >= res.len());
    std::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len());
    *output_len = res.len() as u32;

    sgx_status_t::SGX_SUCCESS
}

use ocall_commands::{CommandResult, OCallCommand};
use ocall_handler::dispatch;

use log::*;
use sgx_types::*;
use std::slice;

#[no_mangle]
pub unsafe extern "C" fn ocall_execute_command(
    command: *const u8,
    command_len: u32,
    output_buf: *mut u8,
    output_buf_maxlen: u32,
    output_len: &mut u32,
) -> sgx_status_t {
    info!("Entering ocall_execute_commands");

    if let Err(e) = validate_const_ptr(command, command_len as usize) {
        return e;
    }

    let cmd: OCallCommand =
        bincode::deserialize(slice::from_raw_parts(command, command_len as usize)).unwrap();

    let (status, result) = match dispatch(cmd) {
        Ok(result) => (sgx_status_t::SGX_SUCCESS, result),
        Err(e) => (
            sgx_status_t::SGX_ERROR_UNEXPECTED,
            CommandResult::CommandError(format!("{:?}", e)),
        ),
    };
    let res = bincode::serialize(&result).unwrap();
    assert!(
        output_buf_maxlen as usize >= res.len(),
        "{} >= {}",
        output_buf_maxlen as usize,
        res.len()
    );
    std::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len());
    *output_len = res.len() as u32;

    status
}

fn validate_const_ptr(ptr: *const u8, ptr_len: usize) -> SgxResult<()> {
    if ptr.is_null() || ptr_len == 0 {
        warn!("Tried to access an empty pointer - ptr.is_null()");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }
    Ok(())
}

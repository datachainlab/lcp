use host_environment::Environment;
use log::*;
use ocall_commands::{CommandResult, OCallCommand};
use once_cell::race::OnceBox;
use sgx_types::sgx_status_t;
use sgx_types::*;
use std::slice;

/// Error indicating that `set_environment` was unable to set the provided Environment
#[derive(Debug, Clone, Copy)]
pub struct SetEnvironmentError;

static HOST_ENVIRONMENT: OnceBox<Environment> = OnceBox::new();

pub fn set_environment(env: Environment) -> Result<(), SetEnvironmentError> {
    HOST_ENVIRONMENT
        .set(Box::new(env))
        .map_err(|_| SetEnvironmentError)
}

pub fn get_environment() -> Option<&'static Environment> {
    HOST_ENVIRONMENT.get()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn ocall_execute_command(
    command: *const u8,
    command_len: u32,
    output_buf: *mut u8,
    output_buf_maxlen: u32,
    output_len: &mut u32,
) -> sgx_types::sgx_status_t {
    debug!("Entering ocall_command_handler");

    if let Err(e) = validate_const_ptr(command, command_len as usize) {
        return e;
    }

    let cmd: OCallCommand = match bincode::serde::decode_borrowed_from_slice(
        unsafe { slice::from_raw_parts(command, command_len as usize) },
        bincode::config::standard(),
    ) {
        Ok(cmd) => cmd,
        Err(e) => {
            error!("failed to bincode::deserialize: {:?}", e);
            return sgx_status_t::SGX_ERROR_UNEXPECTED;
        }
    };

    let (status, result) = match ocall_handler::dispatch(
        HOST_ENVIRONMENT
            .get()
            .expect("you must initialize HOST_ENVIRONMENT before executing the command"),
        cmd,
    ) {
        Ok(result) => (sgx_status_t::SGX_SUCCESS, result),
        Err(e) => (
            sgx_status_t::SGX_ERROR_UNEXPECTED,
            CommandResult::CommandError(format!("{:?}", e)),
        ),
    };

    let res = match bincode::serde::encode_to_vec(&result, bincode::config::standard()) {
        Ok(res) => {
            if res.len() > output_buf_maxlen as usize {
                error!(
                    "output_buf will be overflow: res_len={} output_buf_maxlen={}",
                    res.len(),
                    output_buf_maxlen
                );
                return sgx_status_t::SGX_ERROR_UNEXPECTED;
            }
            res
        }
        Err(e) => {
            error!("failed to bincode::serialize: {:?}", e);
            return sgx_status_t::SGX_ERROR_UNEXPECTED;
        }
    };

    unsafe { std::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len()) };
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

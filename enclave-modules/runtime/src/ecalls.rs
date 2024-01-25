use crate::prelude::*;
use ecall_commands::{CommandResponse, ECallCommand};
use ecall_handler::dispatch;
use enclave_environment::Env;
use enclave_utils::validate_const_ptr;
use log::*;
use once_cell::race::OnceBox;
use sgx_types::sgx_status_t;

/// Error indicating that `set_environment` was unable to set the provided Environment
#[derive(Debug, Clone, Copy)]
pub struct SetEnvironmentError;

static ENCLAVE_ENVIRONMENT: OnceBox<Box<dyn Env>> = OnceBox::new();

pub fn set_environment<E: Env + 'static>(env: E) -> Result<(), SetEnvironmentError> {
    ENCLAVE_ENVIRONMENT
        .set(Box::new(Box::new(env)))
        .map_err(|_| SetEnvironmentError)
}

pub fn ecall_execute_command(
    command: *const u8,
    command_len: u32,
    output_buf: *mut u8,
    output_buf_maxlen: u32,
    output_len: &mut u32,
) -> sgx_status_t {
    debug!("Entering ecall_execute_command");
    validate_const_ptr!(
        command,
        command_len as usize,
        sgx_status_t::SGX_ERROR_UNEXPECTED
    );

    let (status, result) = execute_command(command, command_len);
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
            error!("failed to serialize: result={:?} error={:?}", result, e);
            return sgx_status_t::SGX_ERROR_UNEXPECTED;
        }
    };
    unsafe { core::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len()) };
    *output_len = res.len() as u32;

    status
}

fn execute_command(command: *const u8, command_len: u32) -> (sgx_status_t, CommandResponse) {
    let cmd: ECallCommand = match bincode::serde::decode_borrowed_from_slice(
        unsafe { alloc::slice::from_raw_parts(command, command_len as usize) },
        bincode::config::standard(),
    ) {
        Ok(cmd) => cmd,
        Err(e) => {
            return (
                sgx_status_t::SGX_ERROR_UNEXPECTED,
                CommandResponse::CommandError(format!("failed to bincode::deserialize: {:?}", e)),
            );
        }
    };
    match dispatch(
        ENCLAVE_ENVIRONMENT
            .get()
            .expect("you must initialize ENCLAVE_ENVIRONMENT before executing the command"),
        cmd,
    ) {
        Ok(result) => (sgx_status_t::SGX_SUCCESS, result),
        Err(e) => (
            sgx_status_t::SGX_ERROR_UNEXPECTED,
            CommandResponse::CommandError(format!("{:?}", e)),
        ),
    }
}

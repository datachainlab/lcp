use crate::dispatch;
use crate::traits::OCallHandler;
use host_environment::Environment;
use log::*;
use ocall_commands::{CommandResult, OCallCommand};
use sgx_types::*;
use std::slice;

pub struct HostOCallHandler<'a> {
    env: &'a Environment,
}

impl<'a> HostOCallHandler<'a> {
    pub fn new(env: &'a Environment) -> Self {
        Self { env }
    }
}

impl<'a> OCallHandler for HostOCallHandler<'a> {
    fn handle(
        &self,
        command: *const u8,
        command_len: u32,
        output_buf: *mut u8,
        output_buf_maxlen: u32,
        output_len: &mut u32,
    ) -> sgx_status_t {
        info!("Entering ocall_command_handler");

        if let Err(e) = validate_const_ptr(command, command_len as usize) {
            return e;
        }

        let cmd: OCallCommand =
            bincode::deserialize(unsafe { slice::from_raw_parts(command, command_len as usize) })
                .unwrap();

        let (status, result) = match dispatch(self.env, cmd) {
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
        unsafe { std::ptr::copy_nonoverlapping(res.as_ptr(), output_buf, res.len()) };
        *output_len = res.len() as u32;

        status
    }
}

fn validate_const_ptr(ptr: *const u8, ptr_len: usize) -> SgxResult<()> {
    if ptr.is_null() || ptr_len == 0 {
        warn!("Tried to access an empty pointer - ptr.is_null()");
        return Err(sgx_status_t::SGX_ERROR_UNEXPECTED);
    }
    Ok(())
}

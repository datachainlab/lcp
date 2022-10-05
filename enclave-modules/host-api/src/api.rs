use crate::{
    errors::{HostAPIError as Error, Result},
    ffi,
};
use ocall_commands::{Command, CommandResult};
use sgx_types::*;
use std::vec::Vec;

pub fn execute_command(cmd: Command) -> Result<CommandResult> {
    let cmd_vec = bincode::serialize(&cmd)?;
    let mut ret: sgx_status_t = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut output_len = 0;
    let output_maxlen = 65536;
    let mut output_buf = Vec::with_capacity(output_maxlen);
    let output_ptr = output_buf.as_mut_ptr();

    let result = unsafe {
        ffi::ocall_execute_command(
            &mut ret,
            cmd_vec.as_ptr(),
            cmd_vec.len() as u32,
            output_ptr,
            output_maxlen as u32,
            &mut output_len,
        )
    };

    if result != sgx_status_t::SGX_SUCCESS {
        Err(Error::SGXError(result))
    } else {
        assert!((output_len as usize) < output_maxlen);
        unsafe {
            output_buf.set_len(output_len as usize);
        }
        let res = bincode::deserialize(&output_buf[..output_len as usize])?;
        if ret == sgx_status_t::SGX_SUCCESS {
            Ok(res)
        } else if let CommandResult::CommandError(descr) = res {
            Err(Error::CommandError(ret, descr))
        } else {
            unreachable!()
        }
    }
}

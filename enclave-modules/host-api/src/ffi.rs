use sgx_types::*;

extern "C" {
    pub fn ocall_execute_command(
        ret_val: *mut sgx_status_t,
        command: *const u8,
        command_len: u32,
        output_buf: *mut u8,
        output_buf_maxlen: u32,
        output_len: &mut u32,
    ) -> sgx_status_t;
}

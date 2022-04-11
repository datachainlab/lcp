use sgx_types::{sgx_enclave_id_t, sgx_status_t};

extern "C" {
    pub fn ecall_execute_command(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        command: *const u8,
        command_len: u32,
        output_buf: *mut u8,
        output_buf_maxlen: u32,
        output_len: &mut u32,
    ) -> sgx_status_t;
}

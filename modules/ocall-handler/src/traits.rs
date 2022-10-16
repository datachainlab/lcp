use sgx_types::sgx_status_t;

pub trait OCallHandler {
    fn handle(
        &self,
        command: *const u8,
        command_len: u32,
        output_buf: *mut u8,
        output_buf_maxlen: u32,
        output_len: &mut u32,
    ) -> sgx_status_t;
}

use once_cell::sync::OnceCell;
use sgx_types::sgx_status_t;

/// Error indicating that `set_ocall_handler` was unable to set the provided OCallHandler
#[derive(Debug, Clone, Copy)]
pub struct SetOCallHandlerError;

type OCallHandlerType = Box<dyn OCallHandler + Sync + Send + 'static>;

static OCALL_HANDLER: OnceCell<OCallHandlerType> = OnceCell::new();

pub fn set_ocall_handler(handler: OCallHandlerType) -> Result<(), SetOCallHandlerError> {
    OCALL_HANDLER.set(handler).map_err(|_| SetOCallHandlerError)
}

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

#[no_mangle]
pub unsafe extern "C" fn ocall_execute_command(
    command: *const u8,
    command_len: u32,
    output_buf: *mut u8,
    output_buf_maxlen: u32,
    output_len: &mut u32,
) -> sgx_types::sgx_status_t {
    let handler = OCALL_HANDLER
        .get()
        .expect("ocall handler must be set")
        .as_ref();
    handler.handle(
        command,
        command_len,
        output_buf,
        output_buf_maxlen,
        output_len,
    )
}

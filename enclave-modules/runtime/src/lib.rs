#![no_std]

extern crate alloc;
extern crate sgx_ert;

pub use ecalls::{ecall_execute_command, set_environment};
pub use enclave_environment::{Environment, MapLightClientRegistry};
/// re-export
pub use sgx_trts;
pub use sgx_types;

mod ecalls;
mod errors;

/// `sgx-ert`'s own panic handler is off so this one can log through the host first.
#[cfg(not(test))]
#[allow(unused_variables)]
#[panic_handler]
fn begin_panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    #[cfg(feature = "panic-logging")]
    let msg = alloc::format!("[enclave] panic: {:?}\n", info).into_bytes();
    #[cfg(not(feature = "panic-logging"))]
    let msg = alloc::format!("[enclave] panic\n").into_bytes();
    let _ = host_api::api::execute_command(host_api::ocall_commands::Command::Log(
        host_api::ocall_commands::LogCommand { msg },
    ));
    unsafe { sgx_types::abort() }
}

#[allow(unused_imports)]
mod prelude {
    pub use core::prelude::v1::*;

    // Re-export according to alloc::prelude::v1 because it is not yet stabilized
    // https://doc.rust-lang.org/src/alloc/prelude/v1.rs.html
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;

    pub use alloc::format;
    pub use alloc::vec;

    // Those are exported by default in the std prelude in Rust 2021
    pub use core::convert::{TryFrom, TryInto};
    pub use core::iter::FromIterator;
}

#[macro_export]
macro_rules! setup_runtime {
    ($func:block) => {
        use $crate::sgx_types::cfg_if;

        $crate::sgx_trts::global_ctors_object! {_INIT, _init_func = {
            $crate::set_environment((|| { $func })()).unwrap()
        }}

        #[no_mangle]
        pub unsafe extern "C" fn ecall_execute_command(
            command: *const u8,
            command_len: u32,
            output_buf: *mut u8,
            output_buf_maxlen: u32,
            output_len: &mut u32,
        ) -> u32 {
            $crate::ecall_execute_command(
                command,
                command_len,
                output_buf,
                output_buf_maxlen,
                output_len,
            ) as u32
        }
    };
}

#![no_std]
extern crate alloc;
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

pub use ecalls::{ecall_execute_command, set_environment};
pub use enclave_environment::{Environment, MapLightClientRegistry};
/// re-export
pub use sgx_tstd;

mod ecalls;
mod errors;

#[macro_export]
macro_rules! setup_runtime {
    ($func:block) => {
        use $crate::sgx_tstd::cfg_if;

        $crate::sgx_tstd::global_ctors_object! {_init, _init_func = {
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

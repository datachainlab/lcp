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

pub(crate) use crate::store::get_store;
pub use ecalls::{ecall_execute_command, set_environment};

mod ecalls;
mod errors;
mod store;

#[macro_export]
macro_rules! setup_runtime {
    ($func:block) => {
        sgx_tstd::global_ctors_object! {_init, _init_func = {
            enclave_runtime::set_environment(_env_builder()).unwrap()
        }}

        #[no_mangle]
        pub unsafe extern "C" fn ecall_execute_command(
            command: *const u8,
            command_len: u32,
            output_buf: *mut u8,
            output_buf_maxlen: u32,
            output_len: &mut u32,
        ) -> u32 {
            enclave_runtime::ecall_execute_command(
                command,
                command_len,
                output_buf,
                output_buf_maxlen,
                output_len,
            ) as u32
        }

        fn _env_builder() -> enclave_environment::Environment {
            {
                $func
            }
        }
    };
}

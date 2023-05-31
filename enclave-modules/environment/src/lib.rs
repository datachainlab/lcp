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

pub use environment::Env;
#[cfg(feature = "environment_impl")]
pub use environment_impl::Environment;

mod environment;
#[cfg(feature = "environment_impl")]
mod environment_impl;

pub const fn is_sgx_hw_mode() -> bool {
    let mode = env!("SGX_MODE").as_bytes();
    mode.len() == 2 && mode[0] == 'H' as u8 && mode[1] == 'W' as u8
}

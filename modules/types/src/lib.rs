#![cfg_attr(feature = "sgx", no_std)]
extern crate alloc;
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
pub use any::Any;
pub use errors::{TimeError, TypeError};
pub use height::Height;
pub use time::Time;

mod any;
mod errors;
mod height;
mod time;

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

    #[cfg(feature = "sgx")]
    mod sgx_prelude {
        pub use thiserror_sgx as thiserror;
    }
    #[cfg(feature = "sgx")]
    pub use sgx_prelude::*;
}

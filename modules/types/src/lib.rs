#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use any::Any;
pub use errors::{TimeError, TypeError};
pub use height::Height;
pub use host::ClientId;
pub use sgx::Mrenclave;
pub use time::{Time, MAX_UNIX_TIMESTAMP_SECS};

mod any;
mod errors;
mod height;
mod host;
mod sgx;
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
}

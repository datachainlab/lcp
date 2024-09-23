#![cfg_attr(not(feature = "std"), no_std)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
extern crate alloc;

pub use any::Any;
pub use errors::{TimeError, TypeError};
pub use height::Height;
pub use host::ClientId;
/// re-export
pub use lcp_proto as proto;
pub use sgx::Mrenclave;
pub use time::{nanos_to_duration, Time, MAX_UNIX_TIMESTAMP_NANOS};
pub use transmuter::{deserialize_bytes, serialize_bytes, BytesTransmuter};

mod any;
mod errors;
mod height;
mod host;
mod sgx;
mod time;
mod transmuter;

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

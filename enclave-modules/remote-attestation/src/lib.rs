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

pub use errors::Error;

pub mod attestation;
mod errors;
pub mod report;

#[cfg(feature = "sgx-sw")]
pub mod simulate;

pub const IAS_HOSTNAME: &str = "api.trustedservices.intel.com";

#[cfg(feature = "production")]
pub const SIGRL_SUFFIX: &str = "/sgx/attestation/v4/sigrl/";
#[cfg(not(feature = "production"))]
pub const SIGRL_SUFFIX: &str = "/sgx/dev/attestation/v4/sigrl/";

#[cfg(feature = "production")]
pub const REPORT_SUFFIX: &str = "/sgx/attestation/v4/report";
#[cfg(not(feature = "production"))]
pub const REPORT_SUFFIX: &str = "/sgx/dev/attestation/v4/report";

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

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

pub use errors::Error;
mod errors;

pub use report::{
    AttestationVerificationReport, Quote, ReportData, SignedAttestationVerificationReport,
};
mod report;

#[cfg(feature = "std")]
pub use verification::verify_report;
#[cfg(feature = "std")]
mod verification;

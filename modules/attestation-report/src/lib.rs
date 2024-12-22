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

pub use dcap::{DCAPQuote, Risc0ZKVMProof, ZKDCAPQuote, ZKVMProof};
pub use errors::Error;
pub use ias::{verify_ias_report, IASAttestationVerificationReport, IASSignedReport};
pub use report::{Quote, RAQuote, RAType, ReportData};

pub(crate) mod serde_base64 {
    use crate::prelude::*;
    use base64::{engine::general_purpose::STANDARD as Base64Std, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = Base64Std.encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        Base64Std
            .decode(base64.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

mod dcap;
mod errors;
mod ias;
mod report;

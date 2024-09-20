//! # dcap-qvl
//!
//! This crate implements the quote verification logic for DCAP (Data Center Attestation Primitives) in pure Rust. It supports both SGX (Software Guard Extensions) and TDX (Trust Domain Extensions) quotes.
//!
//! # Features
//! - Verify SGX and TDX quotes
//! - Get collateral from PCCS
//! - Extract information from quotes
//!
//! # Usage
//! Add the following dependency to your `Cargo.toml` file to use this crate:
//! ```toml
//! [dependencies]
//! dcap-qvl = "0.1.0"
//! ```
//!
//! # Example: Get Collateral from PCCS_URL and Verify Quote
//!
//! To get collateral from a PCCS_URL and verify a quote, you can use the following example code:
//! ```no_run
//! use dcap_qvl::collateral::get_collateral;
//! use dcap_qvl::verify::verify;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Get PCCS_URL from environment variable. The URL is like "https://localhost:8081/sgx/certification/v4/".
//!     let pccs_url = std::env::var("PCCS_URL").expect("PCCS_URL is not set");
//!     let quote = std::fs::read("tdx_quote").expect("tdx_quote is not found");
//!     let collateral = get_collateral(&pccs_url, &quote, std::time::Duration::from_secs(10)).await.expect("failed to get collateral");
//!     let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
//!     let tcb = verify(&quote, &collateral, now).expect("failed to verify quote");
//!     println!("{:?}", tcb);
//! }
//! ```

#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

#[macro_use]
extern crate alloc;

use scale::{Decode, Encode};
use scale_info::TypeInfo;

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
use prelude::*;

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidCertificate,
    InvalidSignature,
    CodecError,

    // DCAP
    TCBInfoExpired,
    KeyLengthIsInvalid,
    PublicKeyIsInvalid,
    RsaSignatureIsInvalid,
    DerEncodingError,
    UnsupportedDCAPQuoteVersion,
    UnsupportedDCAPAttestationKeyType,
    UnsupportedQuoteAuthData,
    UnsupportedDCAPPckCertFormat,
    LeafCertificateParsingError,
    CertificateChainIsInvalid,
    CertificateChainIsTooShort,
    IntelExtensionCertificateDecodingError,
    IntelExtensionAmbiguity,
    CpuSvnLengthMismatch,
    CpuSvnDecodingError,
    PceSvnDecodingError,
    PceSvnLengthMismatch,
    FmspcLengthMismatch,
    FmspcDecodingError,
    FmspcMismatch,
    QEReportHashMismatch,
    IsvEnclaveReportSignatureIsInvalid,
    DerDecodingError,
    OidIsMissing,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuoteCollateralV3 {
    pub pck_crl_issuer_chain: String,
    pub pck_crl: String,
    pub tcb_info_issuer_chain: String,
    pub tcb_info: String,
    pub tcb_info_signature: Vec<u8>,
    pub qe_identity_issuer_chain: String,
    pub qe_identity: String,
    pub qe_identity_signature: Vec<u8>,
}

#[cfg(feature = "report")]
pub mod collateral;

mod constants;
mod tcb_info;
mod utils;

pub mod quote;
pub mod verify;

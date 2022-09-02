#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use base64_sgx as base64;
    pub use log_sgx as log;
    pub use pem_sgx as pem;
    pub use rustls_sgx as rustls;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
    pub use webpki_sgx as webpki;
}

pub use errors::AttestationReportError;
pub use report::{
    verify_report, AttestationVerificationReport, EndorsedAttestationVerificationReport, Quote,
};

mod errors;
mod report;

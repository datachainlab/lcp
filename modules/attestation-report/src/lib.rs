#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use base64_sgx as base64;
    pub use pem_sgx as pem;
    pub use rustls_sgx as rustls;
    pub use sgx_tstd as std;
    pub use webpki_sgx as webpki;
}

pub use report::{parse_quote_from_report, verify_report, EndorsedAttestationReport};

mod report;

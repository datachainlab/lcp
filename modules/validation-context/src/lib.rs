#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use sgx_tstd as std;
}

pub use context::ValidationContext;
pub use params::ValidationParams;
pub use predicate::{validation_predicate, ValidationPredicate};

mod context;
mod params;
mod predicate;
pub mod tendermint;

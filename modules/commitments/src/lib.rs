#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}

pub use state::{gen_state_id, gen_state_id_from_any};
pub use types::{ClientCommitment, StateID, ValidityProof};

mod state;
mod types;

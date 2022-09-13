#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
extern crate sgx_types;

// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use log_sgx as log;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}

pub use commitment::{StateCommitment, UpdateClientCommitment};
pub use errors::CommitmentError;
pub use proof::{StateCommitmentProof, UpdateClientCommitmentProof};
pub use state::{
    gen_state_id, gen_state_id_from_any, gen_state_id_from_bytes, StateID, STATE_ID_SIZE,
};

mod commitment;
mod errors;
mod proof;
pub mod prover;
mod state;

#![cfg_attr(feature = "sgx", no_std)]
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;
// re-export module to properly feature gate sgx and regular std environment
#[cfg(feature = "sgx")]
pub mod sgx_reexport_prelude {
    pub use anyhow_sgx as anyhow;
    pub use bincode_sgx as bincode;
    pub use sgx_tstd as std;
    pub use thiserror_sgx as thiserror;
}

pub use crate::errors::StoreError;
pub use crate::store::{CommitStore, KVStore, PersistentStore, VerifiablePersistentStore, Store};
pub use commit::{Commit, CommitID, Revision};
pub use signed_commit::{CommitSigner, CommitVerifier, SignedCommit};

mod commit;
mod errors;
pub mod memory;
#[cfg(feature = "sgx")]
mod sgx_store;
mod signed_commit;
mod store;

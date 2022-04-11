#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

pub use commit::{Commit, CommitID, Sequence};
pub use errors::{Result, StoreError};
pub use store::{CommitStore, KVStore, LoadableStore, Store};

mod commit;
mod errors;
pub mod memory;
mod store;

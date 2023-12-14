#![cfg_attr(not(test), no_std)]
extern crate alloc;

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

pub use context::{TrustingPeriodContext, ValidationContext};
pub use encoder::EthABIEncoder;
pub use errors::Error;
pub use message::{
    CommitmentPrefix, EmittedState, Message, UpdateClientMessage, VerifyMembershipMessage,
};
pub use proof::CommitmentProof;
pub use prover::prove_commitment;
pub use state::{gen_state_id_from_any, gen_state_id_from_bytes, StateID, STATE_ID_SIZE};

mod context;
mod encoder;
mod errors;
mod message;
mod proof;
mod prover;
mod state;

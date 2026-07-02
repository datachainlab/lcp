pub use command::{
    EnclaveCommandAPI, SpeculativeBaseState, SpeculativeEnclaveCommandAPI,
    SpeculativeUpdateClientInput, SpeculativeUpdateClientResponse,
};
pub use primitive::{EnclavePrimitiveAPI, SpeculativeEnclavePrimitiveAPI};
pub use proto::EnclaveProtoAPI;

mod command;
mod primitive;
mod proto;

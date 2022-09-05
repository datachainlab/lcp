use crate::prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum TimeError {
    #[error("SystemTimeError")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("TendermintError: {0}")]
    TendermintError(tendermint::Error),
}

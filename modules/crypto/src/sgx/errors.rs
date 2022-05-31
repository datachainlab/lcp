use crate::sgx_reexport_prelude::*;
use derive_more::Display;

#[derive(Debug, Display, thiserror::Error)]
pub enum SGXCryptoError {
    RandomError,
}

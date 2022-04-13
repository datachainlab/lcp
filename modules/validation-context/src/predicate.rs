#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{params::ValidationParams, ValidationContext};

pub trait ValidationPredicate {
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()>;
}

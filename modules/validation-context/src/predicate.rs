#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{params::ValidationParams, ValidationContext};

pub trait ValidationPredicate {
    // TODO Result's right should be Error type
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()>;
}

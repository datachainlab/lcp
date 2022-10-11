use crate::{
    params::ValidationParams, tendermint::TendermintValidationPredicate, ValidationContext,
};

pub trait ValidationPredicate {
    // TODO Result's right should be Error type
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()>;
}

pub fn validation_predicate(
    vctx: &ValidationContext,
    params: &ValidationParams,
) -> Result<bool, ()> {
    use ValidationParams::*;
    match params {
        Empty => Ok(true),
        Tendermint(_) => TendermintValidationPredicate::predicate(vctx, params),
    }
}

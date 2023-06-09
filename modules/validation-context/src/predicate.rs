use crate::{
    params::ValidationParams, tendermint::TendermintValidationPredicate, ValidationContext,
};

pub trait ValidationPredicate {
    // TODO Result's right should be Error type
    #[allow(clippy::result_unit_err)]
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()>;
}

#[allow(clippy::result_unit_err)]
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

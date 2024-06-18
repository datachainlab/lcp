use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use context::Context;
use crypto::Signer;
use ecall_commands::{
    LightClientResponse, VerifyMembershipInput, VerifyMembershipResponse, VerifyNonMembershipInput,
    VerifyNonMembershipResponse,
};
use light_client::commitments::prove_commitment;
use light_client::LightClientResolver;
use store::KVStore;

pub fn verify_membership<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: VerifyMembershipInput,
) -> Result<LightClientResponse, Error> {
    let ek = ctx.get_enclave_key();
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;

    let res = lc.verify_membership(
        ctx,
        input.client_id,
        input.prefix,
        input.path,
        input.value,
        input.proof.0,
        input.proof.1,
    )?;

    Ok(LightClientResponse::VerifyMembership(
        VerifyMembershipResponse(prove_commitment(ek, res.message.into())?),
    ))
}

pub fn verify_non_membership<R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &mut Context<R, S, K>,
    input: VerifyNonMembershipInput,
) -> Result<LightClientResponse, Error> {
    let ek = ctx.get_enclave_key();
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;

    let res = lc.verify_non_membership(
        ctx,
        input.client_id,
        input.prefix,
        input.path,
        input.proof.0,
        input.proof.1,
    )?;

    Ok(LightClientResponse::VerifyNonMembership(
        VerifyNonMembershipResponse(prove_commitment(ek, res.message.into())?),
    ))
}

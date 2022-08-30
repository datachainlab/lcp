use super::registry::get_light_client_by_client_id;
use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::prove_state_commitment;
use context::Context;
use enclave_commands::{LightClientResult, VerifyClientInput, VerifyClientResult};
use light_client::LightClientSource;
use store::KVStore;

pub fn verify_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: VerifyClientInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();
    let lc = get_light_client_by_client_id::<_, L>(ctx, &input.client_id)?;

    let res = lc.verify_client(
        ctx,
        input.client_id,
        input.target_any_client_state.into(),
        input.prefix,
        input.counterparty_client_id,
        input.proof.0,
        input.proof.1,
    )?;

    Ok(LightClientResult::VerifyClient(VerifyClientResult(
        prove_state_commitment(ek, &res.state_commitment)?,
    )))
}

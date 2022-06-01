use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::prove_state_commitment;
use context::{Context, LightClientReader};
use enclave_commands::{LightClientResult, VerifyClientInput, VerifyClientResult};
use light_client::LightClientSource;
use store::KVStore;

pub fn verify_client<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: VerifyClientInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();

    let client_type = ctx
        .client_type(&input.client_id)
        .map_err(Error::ICS02Error)?;
    let lc = L::get_light_client(&client_type).unwrap();

    let res = lc
        .verify_client(
            ctx,
            input.client_id,
            input.target_any_client_state,
            input.prefix,
            input.counterparty_client_id,
            input.proof.0,
            input.proof.1,
        )
        .map_err(Error::LightClientError)?;

    Ok(LightClientResult::VerifyClient(VerifyClientResult(
        prove_state_commitment(ek, &res.state_commitment).map_err(Error::CommitmentError)?,
    )))
}

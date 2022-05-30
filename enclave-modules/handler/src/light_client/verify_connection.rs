use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::prove_state_commitment;
use context::Context;
use context::{LightClientKeeper, LightClientReader};
use enclave_commands::{LightClientResult, VerifyConnectionInput, VerifyConnectionResult};
use enclave_light_client::LightClientSource;
use store::Store;

pub fn verify_connection<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: VerifyConnectionInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();

    let client_type = ctx
        .client_type(&input.client_id)
        .map_err(Error::ICS02Error)?;
    let lc = L::get_light_client(&client_type).unwrap();

    let res = lc
        .verify_connection(
            ctx,
            input.client_id,
            input.expected_connection,
            input.prefix,
            input.counterparty_connection_id,
            input.proof.0,
            input.proof.1,
        )
        .map_err(Error::LightClientError)?;

    Ok(LightClientResult::VerifyConnection(VerifyConnectionResult(
        prove_state_commitment(ek, &res.state_commitment).map_err(Error::CommitmentError)?,
    )))
}

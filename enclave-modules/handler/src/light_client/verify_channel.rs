use crate::context::Context;
use crate::light_client::LightClientHandlerError as Error;
use commitments::prover::StateCommitmentProver;
use context::{LightClientKeeper, LightClientReader};
use enclave_commands::{LightClientResult, VerifyChannelInput, VerifyChannelResult};
use enclave_light_client::LightClientSource;
use store::Store;

pub fn verify_channel<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: VerifyChannelInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();

    let client_type = ctx
        .client_type(&input.client_id)
        .map_err(Error::ICS02Error)?;
    let lc = L::get_light_client(&client_type).unwrap();

    let res = lc
        .verify_channel(
            ctx,
            input.client_id,
            input.expected_channel,
            input.prefix,
            input.counterparty_port_id,
            input.counterparty_channel_id,
            input.proof.0,
            input.proof.1,
        )
        .map_err(Error::LightClientError)?;

    Ok(LightClientResult::VerifyChannel(VerifyChannelResult(
        ek.prove_state_commitment(&res.state_commitment)
            .map_err(Error::CommitmentError)?,
    )))
}

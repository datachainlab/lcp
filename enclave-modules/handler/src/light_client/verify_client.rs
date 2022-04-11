use crate::context::{Context, LightClientReader};
use crate::light_client::LightClientHandlerError as Error;
use commitments::gen_state_id_from_any;
use enclave_commands::{CommitmentProof, LightClientResult, VerifyClientInput, VerifyClientResult};
use enclave_light_client::LightClientSource;
use enclave_store::Store;

pub fn verify_client<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    input: VerifyClientInput,
) -> Result<LightClientResult, Error> {
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

    let state_id = gen_state_id_from_any(
        &res.trusted_any_client_state,
        &res.trusted_any_consensus_state,
    )
    .map_err(Error::OtherError)?;

    // TODO build a proof
    let commitment_proof = CommitmentProof::default();

    Ok(LightClientResult::VerifyClient(VerifyClientResult(
        commitment_proof,
    )))
}

use crate::light_client::{
    init_client, query_client, update_client, verify_membership, verify_non_membership, Error,
};
use context::Context;
use ecall_commands::{CommandResult, LightClientCommand};
use store::KVStore;

pub fn dispatch<S: KVStore>(
    ctx: &mut Context<S>,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    use LightClientCommand::*;
    let res = match command {
        InitClient(input) => init_client::<S>(ctx, input)?,
        UpdateClient(input) => update_client::<S>(ctx, input)?,
        VerifyMembership(input) => verify_membership::<S>(ctx, input)?,
        VerifyNonMembership(input) => verify_non_membership::<S>(ctx, input)?,

        QueryClient(input) => query_client::<S>(ctx, input)?,
    };
    Ok(CommandResult::LightClient(res))
}

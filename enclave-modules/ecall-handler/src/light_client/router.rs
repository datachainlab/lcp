use crate::light_client::{
    init_client, query_client, update_client, verify_membership, verify_non_membership, Error,
};
use context::Context;
use ecall_commands::{CommandResult, LightClientCommand};
use light_client_registry::LightClientResolver;
use store::KVStore;

pub fn dispatch<R: LightClientResolver, S: KVStore>(
    ctx: &mut Context<R, S>,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    use LightClientCommand::*;
    let res = match command {
        InitClient(input) => init_client(ctx, input)?,
        UpdateClient(input) => update_client(ctx, input)?,
        VerifyMembership(input) => verify_membership(ctx, input)?,
        VerifyNonMembership(input) => verify_non_membership(ctx, input)?,

        QueryClient(input) => query_client(ctx, input)?,
    };
    Ok(CommandResult::LightClient(res))
}

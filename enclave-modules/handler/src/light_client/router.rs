use crate::context::Context;
use crate::light_client::verify_client::verify_client;
use crate::light_client::{init_client, update_client, LightClientHandlerError as Error};
use enclave_light_client::LightClientSource;
use enclave_store::Store;
use enclave_types::commands::{CommandResult, LightClientCommand};

pub fn dispatch<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    use LightClientCommand::*;
    let res = match command {
        InitClient(input) => init_client::<S, L>(ctx, input)?,
        UpdateClient(input) => update_client::<S, L>(ctx, input)?,
        VerifyClient(input) => verify_client::<S, L>(ctx, input)?,
    };
    Ok(CommandResult::LightClient(res))
}

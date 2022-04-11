use crate::context::Context;
use crate::light_client::{init_client, update_client, LightClientHandlerError as Error};
use enclave_light_client::LightClientSource;
use enclave_store::Store;
use enclave_types::commands::{CommandResult, LightClientCommand};

pub fn dispatch<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    let res = match command {
        LightClientCommand::InitClient(input) => init_client::<S, L>(ctx, input)?,
        LightClientCommand::UpdateClient(input) => update_client::<S, L>(ctx, input)?,
    };
    Ok(CommandResult::LightClient(res))
}

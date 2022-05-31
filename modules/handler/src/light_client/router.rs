use crate::light_client::verify_channel::verify_channel;
use crate::light_client::verify_client::verify_client;
use crate::light_client::verify_client_consensus::verify_client_consensus;
use crate::light_client::verify_connection::verify_connection;
use crate::light_client::{
    init_client, update_client, verify_channel, verify_client_consensus, verify_connection,
    LightClientHandlerError as Error,
};
use context::Context;
use enclave_commands::{CommandResult, LightClientCommand};
use light_client::LightClientSource;
use store::Store;

pub fn dispatch<'l, S: Store, L: LightClientSource<'l>>(
    ctx: &mut Context<S>,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    use LightClientCommand::*;
    let res = match command {
        InitClient(input) => init_client::<S, L>(ctx, input)?,
        UpdateClient(input) => update_client::<S, L>(ctx, input)?,
        VerifyClient(input) => verify_client::<S, L>(ctx, input)?,
        VerifyClientConsensus(input) => verify_client_consensus::<S, L>(ctx, input)?,
        VerifyConnection(input) => verify_connection::<S, L>(ctx, input)?,
        VerifyChannel(input) => verify_channel::<S, L>(ctx, input)?,
    };
    Ok(CommandResult::LightClient(res))
}

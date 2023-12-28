use crate::light_client::{
    aggregate_messages, init_client, query_client, update_client, verify_membership,
    verify_non_membership, Error,
};
use context::Context;
use crypto::NopSigner;
use ecall_commands::{
    CommandContext, CommandResult, LightClientCommand, LightClientExecuteCommand,
    LightClientQueryCommand,
};
use enclave_environment::Env;

pub fn dispatch<E: Env>(
    env: E,
    cctx: CommandContext,
    command: LightClientCommand,
) -> Result<CommandResult, Error> {
    let res = match command {
        LightClientCommand::Execute(cmd) => {
            use LightClientExecuteCommand::*;
            let sealed_ek = cctx
                .sealed_ek
                .ok_or(Error::sealed_enclave_key_not_found())?;
            let mut ctx =
                Context::new(env.get_lc_registry(), env.new_store(cctx.tx_id), &sealed_ek);
            match cmd {
                InitClient(input) => init_client(&mut ctx, input)?,
                UpdateClient(input) => update_client(&mut ctx, input)?,
                AggregateMessages(input) => aggregate_messages(&mut ctx, input)?,
                VerifyMembership(input) => verify_membership(&mut ctx, input)?,
                VerifyNonMembership(input) => verify_non_membership(&mut ctx, input)?,
            }
        }
        LightClientCommand::Query(cmd) => {
            use LightClientQueryCommand::*;
            let mut ctx =
                Context::new(env.get_lc_registry(), env.new_store(cctx.tx_id), &NopSigner);
            match cmd {
                QueryClient(input) => query_client(&mut ctx, input)?,
            }
        }
    };
    Ok(CommandResult::LightClient(res))
}

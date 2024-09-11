use crate::enclave_manage::report::create_report;
use crate::enclave_manage::{enclave::generate_enclave_key, Error};
use crate::prelude::*;
use ecall_commands::{
    CommandContext, CommandResponse, EnclaveManageCommand, EnclaveManageResponse,
};

pub fn dispatch(
    cctx: CommandContext,
    command: EnclaveManageCommand,
) -> Result<CommandResponse, Error> {
    use EnclaveManageCommand::*;

    let res = match command {
        GenerateEnclaveKey(input) => CommandResponse::EnclaveManage(
            EnclaveManageResponse::GenerateEnclaveKey(generate_enclave_key(input)?),
        ),
        CreateReport(input) => CommandResponse::EnclaveManage(EnclaveManageResponse::CreateReport(
            create_report(cctx, input)?,
        )),
    };
    Ok(res)
}

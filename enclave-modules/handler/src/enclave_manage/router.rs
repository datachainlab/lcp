use crate::enclave_manage::init_enclave;
use anyhow::Result;
use enclave_commands::{CommandResult, EnclaveManageCommand, EnclaveManageResult};

pub fn dispatch(command: EnclaveManageCommand) -> Result<CommandResult> {
    let res = match command {
        EnclaveManageCommand::InitEnclave(input) => {
            CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(init_enclave(input)?))
        }
    };
    Ok(res)
}

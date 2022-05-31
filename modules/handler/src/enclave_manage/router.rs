use crate::enclave_manage::init_enclave;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
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

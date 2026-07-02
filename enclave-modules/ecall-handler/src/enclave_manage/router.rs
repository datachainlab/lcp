use crate::enclave_manage::{
    enclave::{generate_enclave_key, runtime_info},
    Error,
};
use crate::prelude::*;
use ecall_commands::{CommandResponse, EnclaveManageCommand, EnclaveManageResponse};

pub fn dispatch(command: EnclaveManageCommand) -> Result<CommandResponse, Error> {
    use EnclaveManageCommand::*;

    let res = match command {
        GenerateEnclaveKey(input) => CommandResponse::EnclaveManage(
            EnclaveManageResponse::GenerateEnclaveKey(generate_enclave_key(input)?),
        ),
        RuntimeInfo => {
            CommandResponse::EnclaveManage(EnclaveManageResponse::RuntimeInfo(runtime_info()))
        }
    };
    Ok(res)
}

use crate::enclave_manage::{ias::remote_attestation, init_enclave};
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use anyhow::Result;
use enclave_commands::{CommandParams, CommandResult, EnclaveManageCommand, EnclaveManageResult};

pub fn dispatch(command: EnclaveManageCommand, params: CommandParams) -> Result<CommandResult> {
    use EnclaveManageCommand::*;

    let res = match command {
        InitEnclave(input) => CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(
            init_enclave(input, params)?,
        )),
        IASRemoteAttestation(input) => CommandResult::EnclaveManage(
            EnclaveManageResult::IASRemoteAttestation(remote_attestation(input, params)?),
        ),
    };
    Ok(res)
}

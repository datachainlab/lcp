use crate::enclave_manage::{
    attestation::ias_remote_attestation, init_enclave::init_enclave, Error,
};
use crate::prelude::*;
use ecall_commands::{CommandContext, CommandResult, EnclaveManageCommand, EnclaveManageResult};

pub fn dispatch(
    cctx: CommandContext,
    command: EnclaveManageCommand,
) -> Result<CommandResult, Error> {
    use EnclaveManageCommand::*;

    let res = match command {
        InitEnclave(input) => CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(
            init_enclave(cctx, input)?,
        )),
        IASRemoteAttestation(input) => CommandResult::EnclaveManage(
            EnclaveManageResult::IASRemoteAttestation(ias_remote_attestation(cctx, input)?),
        ),
        #[cfg(feature = "sgx-sw")]
        SimulateRemoteAttestation(input) => {
            CommandResult::EnclaveManage(EnclaveManageResult::SimulateRemoteAttestation(
                crate::enclave_manage::attestation::simulate_remote_attestation(cctx, input)?,
            ))
        }
    };
    Ok(res)
}

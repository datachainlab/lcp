use crate::enclave_manage::{
    attestation::ias_remote_attestation, enclave::generate_enclave_key, Error,
};
use crate::prelude::*;
use ecall_commands::{CommandContext, CommandResult, EnclaveManageCommand, EnclaveManageResult};

pub fn dispatch(
    cctx: CommandContext,
    command: EnclaveManageCommand,
) -> Result<CommandResult, Error> {
    use EnclaveManageCommand::*;

    let res = match command {
        GenerateEnclaveKey(input) => CommandResult::EnclaveManage(
            EnclaveManageResult::GenerateEnclaveKey(generate_enclave_key(input)?),
        ),
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

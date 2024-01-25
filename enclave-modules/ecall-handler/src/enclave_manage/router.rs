use crate::enclave_manage::{
    attestation::ias_remote_attestation, enclave::generate_enclave_key, Error,
};
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
        IASRemoteAttestation(input) => CommandResponse::EnclaveManage(
            EnclaveManageResponse::IASRemoteAttestation(ias_remote_attestation(cctx, input)?),
        ),
        #[cfg(feature = "sgx-sw")]
        SimulateRemoteAttestation(input) => {
            CommandResponse::EnclaveManage(EnclaveManageResponse::SimulateRemoteAttestation(
                crate::enclave_manage::attestation::simulate_remote_attestation(cctx, input)?,
            ))
        }
    };
    Ok(res)
}

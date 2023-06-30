use crate::enclave_manage::{
    attestation::ias_remote_attestation, init_enclave::init_enclave, Error,
};
use crate::prelude::*;
use crypto::sgx::sealing::validate_sealed_enclave_key;
use ecall_commands::{CommandParams, CommandResult, EnclaveManageCommand, EnclaveManageResult};

pub fn dispatch(
    command: EnclaveManageCommand,
    params: CommandParams,
) -> Result<CommandResult, Error> {
    use EnclaveManageCommand::*;

    let res = match command {
        InitEnclave(input) => CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(
            init_enclave(input, params)?,
        )),
        IASRemoteAttestation(input) => {
            validate_sealed_enclave_key(&params.sealed_ek)?;
            CommandResult::EnclaveManage(EnclaveManageResult::IASRemoteAttestation(
                ias_remote_attestation(input, params)?,
            ))
        }
        #[cfg(feature = "sgx-sw")]
        SimulateRemoteAttestation(input) => {
            validate_sealed_enclave_key(&params.sealed_ek)?;
            CommandResult::EnclaveManage(EnclaveManageResult::SimulateRemoteAttestation(
                crate::enclave_manage::attestation::simulate_remote_attestation(input, params)?,
            ))
        }
    };
    Ok(res)
}

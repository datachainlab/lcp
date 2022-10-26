use crate::{EnclavePrimitiveAPI, Result};
use ecall_commands::{
    Command, CommandResult, EnclaveManageCommand, EnclaveManageResult, IASRemoteAttestationInput,
    IASRemoteAttestationResult, InitClientInput, InitClientResult, InitEnclaveInput,
    InitEnclaveResult, LightClientCommand, LightClientResult, QueryClientInput, QueryClientResult,
    UpdateClientInput, UpdateClientResult, VerifyMembershipInput, VerifyMembershipResult,
    VerifyNonMembershipInput, VerifyNonMembershipResult,
};
use store::transaction::CommitStore;

pub trait EnclaveCommandAPI<S: CommitStore>: EnclavePrimitiveAPI<S> {
    /// init_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn init_enclave_key(&self, input: InitEnclaveInput) -> Result<InitEnclaveResult> {
        match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::InitEnclave(input)),
            None,
        )? {
            CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// ias_remote_attestation performs Remote Attestation with IAS(Intel Attestation Service)
    fn ias_remote_attestation(
        &self,
        input: IASRemoteAttestationInput,
    ) -> Result<IASRemoteAttestationResult> {
        match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::IASRemoteAttestation(input)),
            None,
        )? {
            CommandResult::EnclaveManage(EnclaveManageResult::IASRemoteAttestation(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// init_client initializes an ELC instance with given states
    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult> {
        let update_key = Some(input.any_client_state.type_url.clone());
        match self.execute_command(
            Command::LightClient(LightClientCommand::InitClient(input)),
            update_key,
        )? {
            CommandResult::LightClient(LightClientResult::InitClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// update_client updates the ELC instance corresponding to client_id
    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResult> {
        let update_key = Some(input.client_id.to_string());
        match self.execute_command(
            Command::LightClient(LightClientCommand::UpdateClient(input)),
            update_key,
        )? {
            CommandResult::LightClient(LightClientResult::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// verify_membership verifies the existence of the state in the upstream chain and generates the state commitment of its result
    fn verify_membership(&self, input: VerifyMembershipInput) -> Result<VerifyMembershipResult> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::VerifyMembership(input)),
            None,
        )? {
            CommandResult::LightClient(LightClientResult::VerifyMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// verify_non_membership verifies the non-existence of the state in the upstream chain and generates the state commitment of its result
    fn verify_non_membership(
        &self,
        input: VerifyNonMembershipInput,
    ) -> Result<VerifyNonMembershipResult> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::VerifyNonMembership(input)),
            None,
        )? {
            CommandResult::LightClient(LightClientResult::VerifyNonMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// query_client queries the client state and consensus state
    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::QueryClient(input)),
            None,
        )? {
            CommandResult::LightClient(LightClientResult::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

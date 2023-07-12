use crate::{EnclavePrimitiveAPI, Result};
use ecall_commands::{
    Command, CommandResult, EnclaveManageCommand, EnclaveManageResult, GenerateEnclaveKeyInput,
    GenerateEnclaveKeyResult, IASRemoteAttestationInput, IASRemoteAttestationResult,
    InitClientInput, InitClientResult, LightClientCommand, LightClientExecuteCommand,
    LightClientQueryCommand, LightClientResult, QueryClientInput, QueryClientResult,
    UpdateClientInput, UpdateClientResult, VerifyMembershipInput, VerifyMembershipResult,
    VerifyNonMembershipInput, VerifyNonMembershipResult,
};
use store::transaction::CommitStore;

pub trait EnclaveCommandAPI<S: CommitStore>: EnclavePrimitiveAPI<S> {
    /// generate_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn generate_enclave_key(
        &self,
        input: GenerateEnclaveKeyInput,
    ) -> Result<GenerateEnclaveKeyResult> {
        let res = match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::GenerateEnclaveKey(input)),
            None,
        )? {
            CommandResult::EnclaveManage(EnclaveManageResult::GenerateEnclaveKey(res)) => res,
            _ => unreachable!(),
        };
        self.get_key_manager()
            .save(res.pub_key.as_address(), res.sealed_ek.clone())?;
        Ok(res)
    }

    /// ias_remote_attestation performs Remote Attestation with IAS(Intel Attestation Service)
    fn ias_remote_attestation(
        &self,
        input: IASRemoteAttestationInput,
    ) -> Result<IASRemoteAttestationResult> {
        let target_enclave_key = input.target_enclave_key;
        let res = match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::IASRemoteAttestation(input)),
            None,
        )? {
            CommandResult::EnclaveManage(EnclaveManageResult::IASRemoteAttestation(res)) => res,
            _ => unreachable!(),
        };
        self.get_key_manager()
            .save_avr(target_enclave_key, res.report.clone())?;
        Ok(res)
    }

    /// simulate_remote_attestation simulates Remote Attestation
    #[cfg(feature = "sgx-sw")]
    fn simulate_remote_attestation(
        &self,
        input: ecall_commands::SimulateRemoteAttestationInput,
        signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
        signing_cert: Vec<u8>,
    ) -> Result<ecall_commands::SimulateRemoteAttestationResult> {
        use attestation_report::EndorsedAttestationVerificationReport;
        use rsa::signature::{SignatureEncoding, Signer};

        let target_enclave_key = input.target_enclave_key;
        let res = match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::SimulateRemoteAttestation(input)),
            None,
        )? {
            CommandResult::EnclaveManage(EnclaveManageResult::SimulateRemoteAttestation(res)) => {
                res
            }
            _ => unreachable!(),
        };
        let avr_json = res.avr.to_canonical_json().unwrap();
        let signature = signing_key.sign(avr_json.as_bytes()).to_vec();
        let eavr = EndorsedAttestationVerificationReport {
            avr: avr_json,
            signature,
            signing_cert,
        };
        self.get_key_manager().save_avr(target_enclave_key, eavr)?;
        Ok(res)
    }

    /// init_client initializes an ELC instance with given states
    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult> {
        let update_key = Some(input.any_client_state.type_url.clone());
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::InitClient(input),
            )),
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
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::UpdateClient(input),
            )),
            update_key,
        )? {
            CommandResult::LightClient(LightClientResult::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// verify_membership verifies the existence of the state in the upstream chain and generates the state commitment of its result
    fn verify_membership(&self, input: VerifyMembershipInput) -> Result<VerifyMembershipResult> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::VerifyMembership(input),
            )),
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
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::VerifyNonMembership(input),
            )),
            None,
        )? {
            CommandResult::LightClient(LightClientResult::VerifyNonMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// query_client queries the client state and consensus state
    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Query(
                LightClientQueryCommand::QueryClient(input),
            )),
            None,
        )? {
            CommandResult::LightClient(LightClientResult::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

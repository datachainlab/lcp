use crate::{EnclavePrimitiveAPI, Result};
use ecall_commands::{
    AggregateMessagesInput, AggregateMessagesResponse, Command, CommandResponse, CreateReportInput,
    CreateReportResponse, EnclaveManageCommand, EnclaveManageResponse, GenerateEnclaveKeyInput,
    GenerateEnclaveKeyResponse, InitClientInput, InitClientResponse, LightClientCommand,
    LightClientExecuteCommand, LightClientQueryCommand, LightClientResponse, QueryClientInput,
    QueryClientResponse, UpdateClientInput, UpdateClientResponse, VerifyMembershipInput,
    VerifyMembershipResponse, VerifyNonMembershipInput, VerifyNonMembershipResponse,
};
use store::transaction::CommitStore;

pub trait EnclaveCommandAPI<S: CommitStore>: EnclavePrimitiveAPI<S> {
    /// generate_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn generate_enclave_key(
        &self,
        input: GenerateEnclaveKeyInput,
    ) -> Result<GenerateEnclaveKeyResponse> {
        let res = match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::GenerateEnclaveKey(input)),
            None,
        )? {
            CommandResponse::EnclaveManage(EnclaveManageResponse::GenerateEnclaveKey(res)) => res,
            _ => unreachable!(),
        };
        let metadata = self.metadata()?;
        self.get_key_manager().save(
            res.pub_key.as_address(),
            res.sealed_ek.clone(),
            metadata.enclave_css.body.enclave_hash.m.into(),
        )?;
        Ok(res)
    }

    fn create_report(&self, input: CreateReportInput) -> Result<CreateReportResponse> {
        match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::CreateReport(input)),
            None,
        )? {
            CommandResponse::EnclaveManage(EnclaveManageResponse::CreateReport(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    // /// ias_remote_attestation performs Remote Attestation with IAS(Intel Attestation Service)
    // fn ias_remote_attestation(
    //     &self,
    //     input: IASRemoteAttestationInput,
    // ) -> Result<IASRemoteAttestationResponse> {
    //     let target_enclave_key = input.target_enclave_key;
    //     let res = match self.execute_command(
    //         Command::EnclaveManage(EnclaveManageCommand::IASRemoteAttestation(input)),
    //         None,
    //     )? {
    //         CommandResponse::EnclaveManage(EnclaveManageResponse::IASRemoteAttestation(res)) => res,
    //         _ => unreachable!(),
    //     };
    //     self.get_key_manager()
    //         .save_avr(target_enclave_key, res.report.clone())?;
    //     Ok(res)
    // }

    // /// simulate_remote_attestation simulates Remote Attestation
    // #[cfg(feature = "sgx-sw")]
    // fn simulate_remote_attestation(
    //     &self,
    //     input: ecall_commands::SimulateRemoteAttestationInput,
    //     signing_key: rsa::pkcs1v15::SigningKey<sha2::Sha256>,
    //     signing_cert: Vec<u8>,
    // ) -> Result<ecall_commands::SimulateRemoteAttestationResponse> {
    //     use attestation_report::EndorsedAttestationVerificationReport;
    //     use rsa::signature::{SignatureEncoding, Signer};

    //     let target_enclave_key = input.target_enclave_key;
    //     let res = match self.execute_command(
    //         Command::EnclaveManage(EnclaveManageCommand::SimulateRemoteAttestation(input)),
    //         None,
    //     )? {
    //         CommandResponse::EnclaveManage(EnclaveManageResponse::SimulateRemoteAttestation(
    //             res,
    //         )) => res,
    //         _ => unreachable!(),
    //     };
    //     let avr_json = res.avr.to_canonical_json().unwrap();
    //     let signature = signing_key.sign(avr_json.as_bytes()).to_vec();
    //     let eavr = EndorsedAttestationVerificationReport {
    //         avr: avr_json,
    //         signature,
    //         signing_cert,
    //     };
    //     self.get_key_manager().save_avr(target_enclave_key, eavr)?;
    //     Ok(res)
    // }

    /// init_client initializes an ELC instance with given states
    fn init_client(&self, input: InitClientInput) -> Result<InitClientResponse> {
        let update_key = Some(input.any_client_state.type_url.clone());
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::InitClient(input),
            )),
            update_key,
        )? {
            CommandResponse::LightClient(LightClientResponse::InitClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// update_client updates the ELC instance corresponding to client_id
    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResponse> {
        let update_key = Some(input.client_id.to_string());
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::UpdateClient(input),
            )),
            update_key,
        )? {
            CommandResponse::LightClient(LightClientResponse::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn aggregate_messages(
        &self,
        input: AggregateMessagesInput,
    ) -> Result<AggregateMessagesResponse> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::AggregateMessages(input),
            )),
            None,
        )? {
            CommandResponse::LightClient(LightClientResponse::AggregateMessages(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// verify_membership verifies the existence of the state in the upstream chain and generates a message that represents membership of value in the state
    fn verify_membership(&self, input: VerifyMembershipInput) -> Result<VerifyMembershipResponse> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::VerifyMembership(input),
            )),
            None,
        )? {
            CommandResponse::LightClient(LightClientResponse::VerifyMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// verify_non_membership verifies the non-existence of the state in the upstream chain and generates a message that represents non-membership of value in the state
    fn verify_non_membership(
        &self,
        input: VerifyNonMembershipInput,
    ) -> Result<VerifyNonMembershipResponse> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::VerifyNonMembership(input),
            )),
            None,
        )? {
            CommandResponse::LightClient(LightClientResponse::VerifyNonMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    /// query_client queries the client state and consensus state
    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResponse> {
        match self.execute_command(
            Command::LightClient(LightClientCommand::Query(
                LightClientQueryCommand::QueryClient(input),
            )),
            None,
        )? {
            CommandResponse::LightClient(LightClientResponse::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

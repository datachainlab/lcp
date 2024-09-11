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

    /// create_report creates a report for the given target_info
    fn create_report(&self, input: CreateReportInput) -> Result<CreateReportResponse> {
        match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::CreateReport(input)),
            None,
        )? {
            CommandResponse::EnclaveManage(EnclaveManageResponse::CreateReport(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

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

    /// aggregate_messages aggregates the messages and proofs into a single message and proof
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

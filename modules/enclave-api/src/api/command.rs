use crate::{CommitStoreAccessor, EnclavePrimitiveAPI, Result, SpeculativeEnclavePrimitiveAPI};
use attestation_report::QEType;
use ecall_commands::{
    AggregateMessagesInput, AggregateMessagesResponse, Command, CommandResponse,
    EnclaveManageCommand, EnclaveManageResponse, GenerateEnclaveKeyInput,
    GenerateEnclaveKeyResponse, InitClientInput, InitClientResponse, LightClientCommand,
    LightClientExecuteCommand, LightClientQueryCommand, LightClientResponse, QueryClientInput,
    QueryClientResponse, UpdateClientInput, UpdateClientResponse, VerifyMembershipInput,
    VerifyMembershipResponse, VerifyNonMembershipInput, VerifyNonMembershipResponse,
};
use lcp_types::{store_key, Any, Height};
use log::debug;
use store::transaction::{CommitStore, TxAccessor};
use store::TxId;
use store::WriteSet;

#[derive(Debug)]
pub struct SpeculativeUpdateClientInput {
    pub update: UpdateClientInput,
    pub base_state: SpeculativeBaseState,
}

#[derive(Debug, Clone)]
pub struct SpeculativeBaseState {
    pub prev_height: Height,
    pub client_state: Any,
    pub consensus_state: Any,
}

#[derive(Debug)]
pub struct SpeculativeUpdateClientResponse {
    pub response: UpdateClientResponse,
    pub write_set: WriteSet,
}

pub trait EnclaveCommandAPI<S: CommitStore>: EnclavePrimitiveAPI<S> {
    /// generate_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn generate_enclave_key(
        &self,
        input: GenerateEnclaveKeyInput,
        target_qe_type: QEType,
    ) -> Result<GenerateEnclaveKeyResponse> {
        let res = match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::GenerateEnclaveKey(input)),
            None,
        )? {
            CommandResponse::EnclaveManage(EnclaveManageResponse::GenerateEnclaveKey(res)) => res,
            _ => unreachable!(),
        };
        self.get_key_manager()
            .save(res.sealed_ek.clone(), res.report, target_qe_type)?;
        Ok(res)
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
        let update_key = input.client_id.to_string();
        match self.execute_command(
            Command::LightClient(LightClientCommand::Execute(
                LightClientExecuteCommand::UpdateClient(input),
            )),
            Some(update_key),
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

pub trait SpeculativeEnclaveCommandAPI<S: CommitStore + TxAccessor>:
    EnclaveCommandAPI<S> + SpeculativeEnclavePrimitiveAPI<S>
{
    /// speculative_update_client executes `UpdateClient` against an isolated host-side view and
    /// returns both the response and the speculative write set for later stitching.
    fn speculative_update_client(
        &self,
        input: SpeculativeUpdateClientInput,
    ) -> Result<SpeculativeUpdateClientResponse>
    where
        Self: Sized,
    {
        debug!("prepare speculative command with base state");
        let client_id = input.update.client_id.to_string();
        let base_state = input.base_state;

        let cmd = Command::LightClient(LightClientCommand::Execute(
            LightClientExecuteCommand::UpdateClient(input.update),
        ));
        let (res, write_set) = self.execute_command_speculatively_with_seed(cmd, |tx_id| {
            seed_speculative_base_state(self, tx_id, &client_id, &base_state)
        })?;

        match res {
            CommandResponse::LightClient(LightClientResponse::UpdateClient(response)) => {
                Ok(SpeculativeUpdateClientResponse {
                    response,
                    write_set,
                })
            }
            _ => unreachable!(),
        }
    }
}

fn seed_speculative_base_state<S: CommitStore + TxAccessor>(
    enclave: &(impl CommitStoreAccessor<S> + ?Sized),
    tx_id: TxId,
    client_id: &str,
    base_state: &SpeculativeBaseState,
) -> Result<()> {
    let client_state_key = store_key::client_state_bytes(client_id);
    let client_state_value =
        bincode::serde::encode_to_vec(&base_state.client_state, bincode::config::standard())
            .map_err(crate::errors::Error::bincode_encode)?;
    enclave.use_mut_store(|store| store.tx_set(tx_id, client_state_key, client_state_value))?;

    debug_assert!(
        !base_state.consensus_state.type_url.is_empty(),
        "seeded consensus state should carry a concrete type"
    );
    let consensus_state_key = store_key::consensus_state_bytes(client_id, &base_state.prev_height);
    let consensus_state_value =
        bincode::serde::encode_to_vec(&base_state.consensus_state, bincode::config::standard())
            .map_err(crate::errors::Error::bincode_encode)?;
    enclave
        .use_mut_store(|store| store.tx_set(tx_id, consensus_state_key, consensus_state_value))?;
    Ok(())
}

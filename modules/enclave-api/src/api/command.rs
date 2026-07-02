use crate::{CommitStoreAccessor, EnclavePrimitiveAPI, Result, SpeculativeEnclavePrimitiveAPI};
use attestation_report::QEType;
use ecall_commands::{
    AggregateMessagesInput, AggregateMessagesResponse, Command, CommandResponse,
    EnclaveManageCommand, EnclaveManageResponse, EnclaveRuntimeInfo, GenerateEnclaveKeyInput,
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
    /// Effective write set for canonical apply. Entries whose `(key, value)`
    /// match the seeded base state have been removed, so applying this write set
    /// reflects only what speculative UpdateClient actually computed.
    pub write_set: WriteSet,
}

pub trait EnclaveCommandAPI<S: CommitStore>: EnclavePrimitiveAPI<S> {
    /// runtime_info queries runtime SGX/TRTS limits from inside the enclave.
    fn runtime_info(&self) -> Result<EnclaveRuntimeInfo> {
        match self.execute_command(
            Command::EnclaveManage(EnclaveManageCommand::RuntimeInfo),
            None,
        )? {
            CommandResponse::EnclaveManage(EnclaveManageResponse::RuntimeInfo(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

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

        let seed_writes = compute_seed_write_set(&client_id, &base_state)?;
        let cmd = Command::LightClient(LightClientCommand::Execute(
            LightClientExecuteCommand::UpdateClient(input.update),
        ));
        let (res, raw_write_set) = self.execute_command_speculatively_with_seed(cmd, |tx_id| {
            apply_seed_write_set(self, tx_id, &seed_writes)
        })?;
        let effective_write_set = filter_seed_writes(raw_write_set, &seed_writes);

        match res {
            CommandResponse::LightClient(LightClientResponse::UpdateClient(response)) => {
                Ok(SpeculativeUpdateClientResponse {
                    response,
                    write_set: effective_write_set,
                })
            }
            _ => unreachable!(),
        }
    }
}

fn compute_seed_write_set(client_id: &str, base_state: &SpeculativeBaseState) -> Result<WriteSet> {
    let client_state_key = store_key::client_state_bytes(client_id);
    let client_state_value =
        bincode::serde::encode_to_vec(&base_state.client_state, bincode::config::standard())
            .map_err(crate::errors::Error::bincode_encode)?;

    if base_state.consensus_state.type_url.is_empty() {
        return Err(crate::errors::Error::invalid_argument(
            "speculative base_state consensus_state type_url must not be empty".to_string(),
        ));
    }
    let consensus_state_key = store_key::consensus_state_bytes(client_id, &base_state.prev_height);
    let consensus_state_value =
        bincode::serde::encode_to_vec(&base_state.consensus_state, bincode::config::standard())
            .map_err(crate::errors::Error::bincode_encode)?;

    Ok([
        (client_state_key, Some(client_state_value)),
        (consensus_state_key, Some(consensus_state_value)),
    ]
    .into_iter()
    .collect())
}

fn apply_seed_write_set<S: CommitStore + TxAccessor>(
    enclave: &(impl CommitStoreAccessor<S> + ?Sized),
    tx_id: TxId,
    seed_writes: &WriteSet,
) -> Result<()> {
    for (key, value) in seed_writes {
        match value {
            Some(value) => {
                enclave.use_mut_store(|store| store.tx_set(tx_id, key.clone(), value.clone()))?
            }
            None => enclave.use_mut_store(|store| store.tx_remove(tx_id, key))?,
        }
    }
    Ok(())
}

fn filter_seed_writes(write_set: WriteSet, seed_writes: &WriteSet) -> WriteSet {
    write_set
        .into_iter()
        .filter(|(key, value)| seed_writes.get(key) != Some(value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn any(type_url: &str, value: &[u8]) -> Any {
        Any::new(type_url.to_string(), value.to_vec())
    }

    fn base_state() -> SpeculativeBaseState {
        SpeculativeBaseState {
            prev_height: Height::new(0, 10),
            client_state: any("/ibc.mock.ClientState", b"client-10"),
            consensus_state: any("/ibc.mock.ConsensusState", b"consensus-10"),
        }
    }

    #[test]
    fn speculative_update_client_excludes_seeded_consensus_state_from_write_set() {
        let client_id = "07-tendermint-0";
        let seed_writes = compute_seed_write_set(client_id, &base_state()).unwrap();
        let consensus_state_key = store_key::consensus_state_bytes(client_id, &Height::new(0, 10));

        let effective_write_set = filter_seed_writes(seed_writes.clone(), &seed_writes);

        assert!(
            !effective_write_set.contains_key(&consensus_state_key),
            "seeded consensus_state(prev_height) must not be returned as an effective write"
        );
    }

    #[test]
    fn speculative_update_client_keeps_computed_client_state_even_if_seed_provided() {
        let client_id = "07-tendermint-0";
        let seed_writes = compute_seed_write_set(client_id, &base_state()).unwrap();
        let client_state_key = store_key::client_state_bytes(client_id);
        let computed_client_state_value = bincode::serde::encode_to_vec(
            any("/ibc.mock.ClientState", b"client-11"),
            bincode::config::standard(),
        )
        .unwrap();
        let raw_write_set = [(
            client_state_key.clone(),
            Some(computed_client_state_value.clone()),
        )]
        .into_iter()
        .collect();

        let effective_write_set = filter_seed_writes(raw_write_set, &seed_writes);

        assert_eq!(
            effective_write_set.get(&client_state_key),
            Some(&Some(computed_client_state_value))
        );
    }
    #[test]
    fn speculative_update_client_rejects_empty_seeded_consensus_state_type_url() {
        let client_id = "07-tendermint-0";
        let base_state = SpeculativeBaseState {
            consensus_state: any("", b"consensus-10"),
            ..base_state()
        };

        let err = compute_seed_write_set(client_id, &base_state)
            .expect_err("empty consensus_state type_url must be rejected");

        assert!(
            err.to_string()
                .contains("speculative base_state consensus_state type_url must not be empty"),
            "unexpected error: {err}"
        );
    }
}

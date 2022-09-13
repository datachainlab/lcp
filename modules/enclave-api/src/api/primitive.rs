use crate::{ffi, Enclave, EnclaveAPIError as Error, Result};
use enclave_commands::{
    Command, CommandParams, CommandResult, CommitmentProofPair, EnclaveCommand,
    EnclaveManageCommand, EnclaveManageResult, InitClientInput, InitClientResult, InitEnclaveInput,
    InitEnclaveResult, LightClientCommand, LightClientResult, QueryClientInput, QueryClientResult,
    UpdateClientInput, UpdateClientResult, VerifyChannelInput, VerifyChannelResult,
};
use ibc::core::{
    ics04_channel::channel::ChannelEnd,
    ics24_host::identifier::{ChannelId, ClientId, PortId},
};
use lcp_types::Any;
use sgx_types::sgx_status_t;

pub trait EnclavePrimitiveAPI {
    fn execute_command(&self, cmd: &EnclaveCommand) -> Result<CommandResult>;
    fn init_enclave_key(&self, spid: &[u8], ias_key: &[u8]) -> Result<InitEnclaveResult>;
    fn init_client(
        &self,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<InitClientResult>;
    fn update_client(&self, client_id: ClientId, any_header: Any) -> Result<UpdateClientResult>;
    fn verify_channel(
        &self,
        client_id: ClientId,
        expected_channel: ChannelEnd,
        prefix: Vec<u8>,
        counterparty_port_id: PortId,
        counterparty_channel_id: ChannelId,
        proof: CommitmentProofPair,
    ) -> Result<VerifyChannelResult>;

    fn query_client(&self, client_id: ClientId) -> Result<QueryClientResult>;
}

impl EnclavePrimitiveAPI for Enclave {
    fn execute_command(&self, cmd: &EnclaveCommand) -> Result<CommandResult> {
        let mut output_len = 0;
        let output_maxlen = 65536;
        let mut output_buf = Vec::with_capacity(output_maxlen);
        let output_ptr = output_buf.as_mut_ptr();
        let mut ret = sgx_status_t::SGX_SUCCESS;

        let command_bytes = bincode::serialize(cmd)?;
        let result = unsafe {
            ffi::ecall_execute_command(
                self.sgx_enclave.geteid(),
                &mut ret,
                command_bytes.as_ptr(),
                command_bytes.len() as u32,
                output_ptr,
                output_maxlen as u32,
                &mut output_len,
            )
        };
        if result != sgx_status_t::SGX_SUCCESS {
            Err(Error::SGXError(result))
        } else {
            assert!((output_len as usize) < output_maxlen);
            unsafe {
                output_buf.set_len(output_len as usize);
            }
            let res = bincode::deserialize(&output_buf[..output_len as usize])?;
            if ret == sgx_status_t::SGX_SUCCESS {
                Ok(res)
            } else if let CommandResult::CommandError(descr) = res {
                Err(Error::CommandError(ret, descr))
            } else {
                unreachable!()
            }
        }
    }

    fn init_enclave_key(&self, spid: &[u8], ias_key: &[u8]) -> Result<InitEnclaveResult> {
        let cmd = Command::EnclaveManage(EnclaveManageCommand::InitEnclave(InitEnclaveInput {
            spid: spid.to_vec(),
            ias_key: ias_key.to_vec(),
        }));
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            cmd,
        ))? {
            CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn init_client(
        &self,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<InitClientResult> {
        let cmd = Command::LightClient(LightClientCommand::InitClient(InitClientInput {
            any_client_state: any_client_state.into(),
            any_consensus_state: any_consensus_state.into(),
            current_timestamp: Self::current_timestamp(),
        }));
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            cmd,
        ))? {
            CommandResult::LightClient(LightClientResult::InitClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn update_client(&self, client_id: ClientId, any_header: Any) -> Result<UpdateClientResult> {
        let cmd = Command::LightClient(LightClientCommand::UpdateClient(UpdateClientInput {
            client_id,
            any_header: any_header.into(),
            current_timestamp: Self::current_timestamp(),
        }));
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            cmd,
        ))? {
            CommandResult::LightClient(LightClientResult::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_channel(
        &self,
        client_id: ClientId,
        expected_channel: ChannelEnd,
        prefix: Vec<u8>,
        counterparty_port_id: PortId,
        counterparty_channel_id: ChannelId,
        proof: CommitmentProofPair,
    ) -> Result<VerifyChannelResult> {
        let cmd = Command::LightClient(LightClientCommand::VerifyChannel(VerifyChannelInput {
            client_id,
            expected_channel,
            prefix,
            counterparty_port_id,
            counterparty_channel_id,
            proof,
        }));
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            cmd,
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyChannel(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn query_client(&self, client_id: ClientId) -> Result<QueryClientResult> {
        let cmd = Command::LightClient(LightClientCommand::QueryClient(QueryClientInput {
            client_id,
        }));
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            cmd,
        ))? {
            CommandResult::LightClient(LightClientResult::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

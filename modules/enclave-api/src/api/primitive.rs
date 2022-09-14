use crate::{ffi, Enclave, EnclaveAPIError as Error, Result};
use enclave_commands::{
    Command, CommandParams, CommandResult, EnclaveCommand, EnclaveManageCommand,
    EnclaveManageResult, InitClientInput, InitClientResult, InitEnclaveInput, InitEnclaveResult,
    LightClientCommand, LightClientResult, QueryClientInput, QueryClientResult, UpdateClientInput,
    UpdateClientResult, VerifyChannelInput, VerifyChannelResult, VerifyClientConsensusInput,
    VerifyClientConsensusResult, VerifyClientInput, VerifyClientResult, VerifyConnectionInput,
    VerifyConnectionResult,
};
use sgx_types::sgx_status_t;

pub trait EnclavePrimitiveAPI {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: &EnclaveCommand) -> Result<CommandResult>;

    /// init_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn init_enclave_key(&self, input: InitEnclaveInput) -> Result<InitEnclaveResult>;

    /// init_client initializes an ELC instance with given states
    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult>;

    /// update_client updates the ELC instance corresponding to client_id
    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResult>;

    /// verify_client verifies the client state of the upstream chain and generates the state commitment of its result
    fn verify_client(&self, input: VerifyClientInput) -> Result<VerifyClientResult>;

    /// verify_client_consensus verifies the consensus state of the upstream chain and generates the state commitment of its result
    fn verify_client_consensus(
        &self,
        input: VerifyClientConsensusInput,
    ) -> Result<VerifyClientConsensusResult>;

    /// verify_connection verifies the connection state of the upstream chain and generates the state commitment of its result
    fn verify_connection(&self, input: VerifyConnectionInput) -> Result<VerifyConnectionResult>;

    /// verify_channel verifies the channel state of the upstream chain and generates the state commitment of its result
    fn verify_channel(&self, input: VerifyChannelInput) -> Result<VerifyChannelResult>;

    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult>;
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

    fn init_enclave_key(&self, input: InitEnclaveInput) -> Result<InitEnclaveResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::EnclaveManage(EnclaveManageCommand::InitEnclave(input)),
        ))? {
            CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::InitClient(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::InitClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::UpdateClient(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_client(&self, input: VerifyClientInput) -> Result<VerifyClientResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::VerifyClient(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_client_consensus(
        &self,
        input: VerifyClientConsensusInput,
    ) -> Result<VerifyClientConsensusResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::VerifyClientConsensus(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyClientConsensus(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_connection(&self, input: VerifyConnectionInput) -> Result<VerifyConnectionResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::VerifyConnection(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyConnection(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_channel(&self, input: VerifyChannelInput) -> Result<VerifyChannelResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::VerifyChannel(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyChannel(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult> {
        match self.execute_command(&EnclaveCommand::new(
            CommandParams::new(self.home.clone()),
            Command::LightClient(LightClientCommand::QueryClient(input)),
        ))? {
            CommandResult::LightClient(LightClientResult::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

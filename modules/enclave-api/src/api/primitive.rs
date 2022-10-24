use crate::{ffi, Enclave, Error, Result};
use ecall_commands::{
    Command, CommandParams, CommandResult, ECallCommand, EnclaveManageCommand, EnclaveManageResult,
    IASRemoteAttestationInput, IASRemoteAttestationResult, InitClientInput, InitClientResult,
    InitEnclaveInput, InitEnclaveResult, LightClientCommand, LightClientResult, QueryClientInput,
    QueryClientResult, UpdateClientInput, UpdateClientResult, VerifyMembershipInput,
    VerifyMembershipResult, VerifyNonMembershipInput, VerifyNonMembershipResult,
};
use sgx_types::sgx_status_t;

pub trait EnclavePrimitiveAPI {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: Command) -> Result<CommandResult>;

    /// init_enclave_key generates a new key and perform remote attestation to generates an AVR
    fn init_enclave_key(&self, input: InitEnclaveInput) -> Result<InitEnclaveResult>;

    /// ias_remote_attestation performs Remote Attestation with IAS(Intel Attestation Service)
    fn ias_remote_attestation(
        &self,
        input: IASRemoteAttestationInput,
    ) -> Result<IASRemoteAttestationResult>;

    /// init_client initializes an ELC instance with given states
    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult>;

    /// update_client updates the ELC instance corresponding to client_id
    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResult>;

    /// verify_membership verifies the existence of the state in the upstream chain and generates the state commitment of its result
    fn verify_membership(&self, input: VerifyMembershipInput) -> Result<VerifyMembershipResult>;

    /// verify_non_membership verifies the non-existence of the state in the upstream chain and generates the state commitment of its result
    fn verify_non_membership(
        &self,
        input: VerifyNonMembershipInput,
    ) -> Result<VerifyNonMembershipResult>;

    /// query_client queries the client state and consensus state
    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult>;
}

impl EnclavePrimitiveAPI for Enclave {
    fn execute_command(&self, cmd: Command) -> Result<CommandResult> {
        let mut output_len = 0;
        let output_maxlen = 65536;
        let mut output_buf = Vec::with_capacity(output_maxlen);
        let output_ptr = output_buf.as_mut_ptr();
        let mut ret = sgx_status_t::SGX_SUCCESS;

        let env = host::get_environment().unwrap();
        let tx_id = env.store.write().unwrap().begin().unwrap();

        let ecmd = ECallCommand::new(CommandParams::new(self.home.clone(), tx_id), cmd);
        let command_bytes = bincode::serialize(&ecmd).map_err(Error::bincode)?;
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
        let mut store = env.store.write().unwrap();
        if result != sgx_status_t::SGX_SUCCESS {
            store.rollback(tx_id);
            Err(Error::sgx_error(result))
        } else {
            assert!((output_len as usize) < output_maxlen);
            unsafe {
                output_buf.set_len(output_len as usize);
            }
            let res =
                bincode::deserialize(&output_buf[..output_len as usize]).map_err(Error::bincode)?;
            if ret == sgx_status_t::SGX_SUCCESS {
                store.commit(tx_id).unwrap();
                Ok(res)
            } else if let CommandResult::CommandError(descr) = res {
                store.rollback(tx_id);
                Err(Error::command(ret, descr))
            } else {
                unreachable!()
            }
        }
    }

    fn init_enclave_key(&self, input: InitEnclaveInput) -> Result<InitEnclaveResult> {
        match self.execute_command(Command::EnclaveManage(EnclaveManageCommand::InitEnclave(
            input,
        )))? {
            CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn ias_remote_attestation(
        &self,
        input: IASRemoteAttestationInput,
    ) -> Result<IASRemoteAttestationResult> {
        match self.execute_command(Command::EnclaveManage(
            EnclaveManageCommand::IASRemoteAttestation(input),
        ))? {
            CommandResult::EnclaveManage(EnclaveManageResult::IASRemoteAttestation(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn init_client(&self, input: InitClientInput) -> Result<InitClientResult> {
        match self.execute_command(Command::LightClient(LightClientCommand::InitClient(input)))? {
            CommandResult::LightClient(LightClientResult::InitClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn update_client(&self, input: UpdateClientInput) -> Result<UpdateClientResult> {
        match self.execute_command(Command::LightClient(LightClientCommand::UpdateClient(
            input,
        )))? {
            CommandResult::LightClient(LightClientResult::UpdateClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_membership(&self, input: VerifyMembershipInput) -> Result<VerifyMembershipResult> {
        match self.execute_command(Command::LightClient(LightClientCommand::VerifyMembership(
            input,
        )))? {
            CommandResult::LightClient(LightClientResult::VerifyMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn verify_non_membership(
        &self,
        input: VerifyNonMembershipInput,
    ) -> Result<VerifyNonMembershipResult> {
        match self.execute_command(Command::LightClient(
            LightClientCommand::VerifyNonMembership(input),
        ))? {
            CommandResult::LightClient(LightClientResult::VerifyNonMembership(res)) => Ok(res),
            _ => unreachable!(),
        }
    }

    fn query_client(&self, input: QueryClientInput) -> Result<QueryClientResult> {
        match self.execute_command(Command::LightClient(LightClientCommand::QueryClient(input)))? {
            CommandResult::LightClient(LightClientResult::QueryClient(res)) => Ok(res),
            _ => unreachable!(),
        }
    }
}

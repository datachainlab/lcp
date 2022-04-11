use std::time::{SystemTime, UNIX_EPOCH};

use crate::{ffi, EnclaveAPIError as Error, Result};
use enclave_commands::{
    Command, CommandResult, EnclaveManageCommand, InitClientInput, InitEnclaveInput,
    LightClientCommand, UpdateClientInput,
};
use ibc::core::ics24_host::identifier::ClientId;
use prost_types::Any;
use sgx_types::{sgx_enclave_id_t, sgx_status_t};
use sgx_urts::SgxEnclave;

#[derive(Clone, Debug, Default)]
pub struct Enclave {
    eid: sgx_enclave_id_t,
    sgx_enclave: SgxEnclave,
}

pub trait EnclaveAPI {
    fn execute_command(&self, cmd: &Command) -> Result<CommandResult>;
    fn init_enclave_key(&self, spid: &[u8], ias_key: &[u8]) -> Result<CommandResult>;
    fn init_client(
        &self,
        client_type: &str,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CommandResult>;
    fn update_client(&self, client_id: ClientId, any_header: Any) -> Result<CommandResult>;
}

impl Enclave {
    pub fn new(sgx_enclave: SgxEnclave) -> Self {
        Enclave {
            eid: sgx_enclave.geteid(),
            sgx_enclave,
        }
    }

    pub fn destroy(self) {
        self.sgx_enclave.destroy()
    }

    pub fn current_timestamp() -> u64 {
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        current_timestamp.as_nanos() as u64
    }
}

impl EnclaveAPI for Enclave {
    fn execute_command(&self, cmd: &Command) -> Result<CommandResult> {
        let mut output_len = 0;
        let output_maxlen = 1024;
        let mut output_buf = Vec::with_capacity(output_maxlen);
        let output_ptr = output_buf.as_mut_ptr();
        let mut ret = sgx_status_t::SGX_SUCCESS;

        let command_bytes = bincode::serialize(cmd).unwrap();
        let result = unsafe {
            ffi::ecall_execute_command(
                self.eid,
                &mut ret,
                command_bytes.as_ptr(),
                command_bytes.len() as u32,
                output_ptr,
                output_maxlen as u32,
                &mut output_len,
            )
        };
        if ret != sgx_status_t::SGX_SUCCESS {
            Err(Error::SGXError(ret))
        } else if result != sgx_status_t::SGX_SUCCESS {
            Err(Error::SGXError(result))
        } else {
            assert!((output_len as usize) < output_maxlen);
            unsafe {
                output_buf.set_len(output_len as usize);
            }
            Ok(bincode::deserialize(&output_buf[..output_len as usize])
                .map_err(Error::BincodeError)?)
        }
    }

    fn init_enclave_key(&self, spid: &[u8], ias_key: &[u8]) -> Result<CommandResult> {
        let command = Command::EnclaveManage(EnclaveManageCommand::InitEnclave(InitEnclaveInput {
            spid: spid.to_vec(),
            ias_key: ias_key.to_vec(),
        }));
        self.execute_command(&command)
    }

    fn init_client(
        &self,
        client_type: &str,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CommandResult> {
        let command = Command::LightClient(LightClientCommand::InitClient(InitClientInput {
            current_timestamp: Self::current_timestamp(),
            client_type: client_type.into(),
            any_client_state,
            any_consensus_state,
        }));
        self.execute_command(&command)
    }

    fn update_client(&self, client_id: ClientId, any_header: Any) -> Result<CommandResult> {
        let command = Command::LightClient(LightClientCommand::UpdateClient(UpdateClientInput {
            client_id,
            any_header,
            current_timestamp: Self::current_timestamp(),
        }));
        self.execute_command(&command)
    }
}

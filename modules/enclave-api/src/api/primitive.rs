use crate::{
    enclave::{EnclaveInfo, HostStoreTxManager},
    ffi, Error, Result,
};
use ecall_commands::{Command, CommandContext, CommandResponse, ECallCommand, EnclaveKeySelector};
use lcp_types::Time;
use log::*;
use sgx_types::{sgx_enclave_id_t, sgx_status_t};
use store::transaction::{CommitStore, Tx};

pub trait EnclavePrimitiveAPI<S: CommitStore>: EnclaveInfo + HostStoreTxManager<S> {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: Command, update_key: Option<String>) -> Result<CommandResponse> {
        debug!(
            "prepare command: inner={:?} update_key={:?}",
            cmd, update_key
        );
        let current_timestamp = Time::now();
        let tx = self.begin_tx(update_key)?;

        let cctx = match cmd.get_enclave_key() {
            Some(addr) => {
                let ski = self.get_key_manager().load(addr)?;
                CommandContext::new(current_timestamp, Some(ski.sealed_ek), tx.get_id())
            }
            None => CommandContext::new(current_timestamp, None, tx.get_id()),
        };

        let ecmd = ECallCommand::new(cctx, cmd);
        debug!("try to execute command: {:?}", ecmd);
        match raw_execute_command(self.get_eid(), ecmd) {
            Ok(res) => {
                self.commit_tx(tx)?;
                debug!("execute_command succeeded: res={:?}", res);
                Ok(res)
            }
            Err(e) => {
                self.rollback_tx(tx);
                debug!("execute_command failed: err={:?}", e);
                Err(e)
            }
        }
    }
}

fn raw_execute_command(eid: sgx_enclave_id_t, cmd: ECallCommand) -> Result<CommandResponse> {
    let mut output_len = 0;
    let output_maxlen = 65536;
    let mut output_buf = Vec::with_capacity(output_maxlen);
    let output_ptr = output_buf.as_mut_ptr();
    let mut ret = sgx_status_t::SGX_SUCCESS;

    let command_bytes = bincode::serde::encode_to_vec(&cmd, bincode::config::standard())
        .map_err(Error::bincode_encode)?;
    let result = unsafe {
        ffi::ecall_execute_command(
            eid,
            &mut ret,
            command_bytes.as_ptr(),
            command_bytes.len() as u32,
            output_ptr,
            output_maxlen as u32,
            &mut output_len,
        )
    };
    if result != sgx_status_t::SGX_SUCCESS {
        Err(Error::sgx_error(result))
    } else {
        assert!((output_len as usize) < output_maxlen);
        unsafe {
            output_buf.set_len(output_len as usize);
        }
        let res = bincode::serde::decode_borrowed_from_slice(
            &output_buf[..output_len as usize],
            bincode::config::standard(),
        )
        .map_err(Error::bincode_decode)?;

        if ret == sgx_status_t::SGX_SUCCESS {
            Ok(res)
        } else if let CommandResponse::CommandError(descr) = res {
            Err(Error::command(ret, descr))
        } else {
            unreachable!()
        }
    }
}

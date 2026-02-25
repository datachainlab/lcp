use crate::{
    enclave::{EnclaveInfo, HostStoreTxManager},
    ffi, Error, Result,
};
use core::fmt::Write;
use ecall_commands::{
    Command, CommandContext, CommandResponse, ECallCommand, EnclaveKeySelector, LightClientCommand,
    LightClientExecuteCommand,
};
use lcp_types::Time;
use log::*;
use sgx_types::{sgx_enclave_id_t, sgx_status_t};
use sha2::{Digest, Sha256};
use store::transaction::{CommitStore, Tx};

pub trait EnclavePrimitiveAPI<S: CommitStore>: EnclaveInfo + HostStoreTxManager<S> {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: Command, update_key: Option<String>) -> Result<CommandResponse> {
        let update_client_probe = extract_update_client_probe(&cmd);
        debug!(
            "prepare command: inner={:?} update_key={:?}",
            cmd, update_key
        );
        if let Some(probe) = update_client_probe.as_ref() {
            debug!(
                "prepare update_client command: client_id={} include_state={} type_url={} header_len={} header_sha256={}",
                probe.client_id,
                probe.include_state,
                probe.type_url,
                probe.header_len,
                probe.header_sha256
            );
        }
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
                if let Some(probe) = update_client_probe.as_ref() {
                    debug!(
                        "execute_command succeeded (update_client): client_id={} include_state={} type_url={} header_len={} header_sha256={} res={:?}",
                        probe.client_id,
                        probe.include_state,
                        probe.type_url,
                        probe.header_len,
                        probe.header_sha256,
                        res
                    );
                } else {
                    debug!("execute_command succeeded: res={:?}", res);
                }
                Ok(res)
            }
            Err(e) => {
                self.rollback_tx(tx);
                if let Some(probe) = update_client_probe.as_ref() {
                    error!(
                        "execute_command failed (update_client): client_id={} include_state={} type_url={} header_len={} header_sha256={} err={:?}",
                        probe.client_id,
                        probe.include_state,
                        probe.type_url,
                        probe.header_len,
                        probe.header_sha256,
                        e
                    );
                } else {
                    debug!("execute_command failed: err={:?}", e);
                }
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

#[derive(Debug)]
struct UpdateClientProbe {
    client_id: String,
    include_state: bool,
    type_url: String,
    header_len: usize,
    header_sha256: String,
}

fn extract_update_client_probe(cmd: &Command) -> Option<UpdateClientProbe> {
    let input = match cmd {
        Command::LightClient(LightClientCommand::Execute(
            LightClientExecuteCommand::UpdateClient(input),
        )) => input,
        _ => return None,
    };

    Some(UpdateClientProbe {
        client_id: input.client_id.to_string(),
        include_state: input.include_state,
        type_url: input.any_header.type_url.clone(),
        header_len: input.any_header.value.len(),
        header_sha256: sha256_hex(&input.any_header.value),
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}

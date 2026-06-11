use crate::{
    enclave::{EnclaveInfo, HostStoreTxManager},
    ffi, Error, Result,
};
use ecall_commands::{Command, CommandContext, CommandResponse, ECallCommand, EnclaveKeySelector};
use lcp_types::Time;
use log::*;
use sgx_types::{sgx_enclave_id_t, sgx_status_t};
use store::transaction::{CommitStore, Tx, TxAccessor};
use store::TxId;
use store::WriteSet;

pub trait EnclavePrimitiveAPI<S: CommitStore>: EnclaveInfo + HostStoreTxManager<S> {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: Command, update_key: Option<String>) -> Result<CommandResponse> {
        debug!(
            "prepare command: inner={:?} update_key={:?}",
            cmd, update_key
        );
        let tx = self.begin_tx(update_key)?;
        match execute_prepared_command(self, cmd, tx.get_id()) {
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

pub trait SpeculativeEnclavePrimitiveAPI<S: CommitStore + TxAccessor>:
    EnclavePrimitiveAPI<S>
{
    /// execute_command_speculatively runs a command against an isolated host-side view and returns
    /// the response together with the speculative write set instead of committing it.
    fn execute_command_speculatively(&self, cmd: Command) -> Result<(CommandResponse, WriteSet)> {
        self.execute_command_speculatively_with_seed(cmd, |_| Ok(()))
    }

    /// execute_command_speculatively_with_seed runs a command against an isolated host-side view
    /// after giving the caller a chance to seed the speculative transaction.
    fn execute_command_speculatively_with_seed(
        &self,
        cmd: Command,
        seed: impl FnOnce(TxId) -> Result<()>,
    ) -> Result<(CommandResponse, WriteSet)> {
        debug!("prepare speculative command: inner={:?}", cmd);
        let tx = self.begin_speculative_tx()?;
        if let Err(e) = seed(tx.get_id()) {
            self.rollback_tx(tx);
            return Err(e);
        }
        match execute_prepared_command(self, cmd, tx.get_id()) {
            Ok(res) => {
                let writes = self.use_mut_store(|store| store.take_write_set(tx))?;
                debug!(
                    "execute_command_speculatively succeeded: res={:?} writes={}",
                    res,
                    writes.len()
                );
                Ok((res, writes))
            }
            Err(e) => {
                self.rollback_tx(tx);
                debug!("execute_command_speculatively failed: err={:?}", e);
                Err(e)
            }
        }
    }
}

impl<T, S> SpeculativeEnclavePrimitiveAPI<S> for T
where
    S: CommitStore + TxAccessor,
    T: EnclavePrimitiveAPI<S>,
{
}

/// execute_prepared_command runs a command against an already begun host-store transaction.
fn execute_prepared_command(
    enclave: &(impl EnclaveInfo + ?Sized),
    cmd: Command,
    tx_id: TxId,
) -> Result<CommandResponse> {
    // Concurrency is now bounded structurally by `service::EcallPool`. The
    // caller is expected to run on one of the pool's permanent ECALL
    // workers, keeping TCSPolicy=BIND bindings within the pool size.
    let current_timestamp = Time::now();
    let cctx = match cmd.get_enclave_key() {
        Some(addr) => {
            let ski = enclave.get_key_manager().load(addr)?;
            CommandContext::new(current_timestamp, Some(ski.sealed_ek), tx_id)
        }
        None => CommandContext::new(current_timestamp, None, tx_id),
    };

    let ecmd = ECallCommand::new(cctx, cmd);
    debug!("try to execute command: {:?}", ecmd);
    raw_execute_command(enclave.get_eid(), ecmd)
}

pub(crate) fn raw_execute_command(
    eid: sgx_enclave_id_t,
    cmd: ECallCommand,
) -> Result<CommandResponse> {
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

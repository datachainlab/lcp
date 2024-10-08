use crate::prelude::*;
use crate::{errors::Error, ffi};
use ocall_commands::{Command, CommandResult};
use sgx_types::*;

pub fn execute_command(cmd: Command) -> Result<CommandResult, Error> {
    let cmd_vec = bincode::serde::encode_to_vec(&cmd, bincode::config::standard())
        .map_err(Error::bincode_encode)?;
    let mut ret: sgx_status_t = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut output_len = 0;
    let output_maxlen = 65536;
    let mut output_buf = Vec::with_capacity(output_maxlen);
    let output_ptr = output_buf.as_mut_ptr();

    let result = unsafe {
        ffi::ocall_execute_command(
            &mut ret,
            cmd_vec.as_ptr(),
            cmd_vec.len() as u32,
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
        } else if let CommandResult::CommandError(descr) = res {
            Err(Error::command(ret, descr))
        } else {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enclave_api::{CommitStoreAccessor, HostStoreTxManager};
    use host::environment::{Environment, HostStore};
    use ocall_commands::{LogCommand, StoreCommand, StoreResult};
    use std::sync::{Arc, RwLock};
    use store::{
        host::IntoCommitStore,
        transaction::{CommitStore, Tx},
        KVStore,
    };
    use tempfile::TempDir;

    struct TestEnv;

    impl<S> CommitStoreAccessor<S> for TestEnv
    where
        S: CommitStore,
        HostStore: IntoCommitStore<S>,
    {
        fn use_mut_store<T>(&self, f: impl FnOnce(&mut S) -> T) -> T {
            let mut store = host::get_environment().unwrap().get_mut_store();
            store.apply(f)
        }
    }

    impl<S> HostStoreTxManager<S> for TestEnv
    where
        S: CommitStore,
        HostStore: IntoCommitStore<S>,
    {
    }

    #[test]
    fn test_execute_command() {
        let tmp_dir = TempDir::new().unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();
        let mstore = Arc::new(RwLock::new(HostStore::Memory(Default::default())));
        host::set_environment(Environment::new(home.into(), mstore.clone())).unwrap();

        {
            let res = execute_command(LogCommand::new(b"TEST".to_vec()).into());
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), CommandResult::Log);
        }
        {
            let tx = TestEnv.begin_tx(None).unwrap();
            let res = execute_command(
                StoreCommand::Set(tx.get_id(), b"k0".to_vec(), b"v0".to_vec()).into(),
            );
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), CommandResult::Store(StoreResult::Set));
            let res = execute_command(StoreCommand::Get(tx.get_id(), b"k0".to_vec()).into());
            assert!(res.is_ok());
            assert_eq!(
                res.unwrap(),
                CommandResult::Store(StoreResult::Get(Some(b"v0".to_vec())))
            );
            TestEnv.commit_tx(tx).unwrap();
            mstore.write().unwrap().apply(|store| {
                let v = store.get(b"k0");
                assert_eq!(v, Some(b"v0".to_vec()));
            });
        }
    }
}

use crate::opts::Opts;
use anyhow::{bail, Result};
use enclave_api::{Enclave, EnclaveProtoAPI};
use keymanager::EnclaveKeyManager;
use std::path::PathBuf;
use store::transaction::CommitStore;

pub(crate) fn build_enclave_loader<S: CommitStore>(
) -> impl FnOnce(&Opts, Option<&PathBuf>, bool) -> Result<Enclave<S>>
where
    Enclave<S>: EnclaveProtoAPI<S>,
{
    |opts, path, debug| {
        let path = if let Some(path) = path {
            path.clone()
        } else {
            opts.default_enclave()
        };
        let env = host::get_environment().unwrap();
        let km = EnclaveKeyManager::new(&env.home)?;
        match Enclave::create(&path, debug, km, env.store.clone()) {
            Ok(enclave) => Ok(enclave),
            Err(x) => {
                bail!(
                    "Init Enclave Failed: status={} path={:?}",
                    x.as_str(),
                    path.as_path()
                );
            }
        }
    }
}

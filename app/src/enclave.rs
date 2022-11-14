use crate::opts::Opts;
use anyhow::{bail, Result};
use enclave_api::{Enclave, EnclaveProtoAPI};
use std::path::PathBuf;
use store::transaction::CommitStore;

pub(crate) fn build_enclave_loader<'e, S: CommitStore>(
) -> impl FnOnce(&Opts, Option<&PathBuf>) -> Result<Enclave<'e, S>>
where
    Enclave<'e, S>: EnclaveProtoAPI<S>,
{
    |opts, path| {
        let path = if let Some(path) = path {
            path.clone()
        } else {
            opts.default_enclave()
        };
        match host::load_enclave(&path) {
            Ok(enclave) => Ok(Enclave::<S>::new(enclave, host::get_environment().unwrap())),
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
